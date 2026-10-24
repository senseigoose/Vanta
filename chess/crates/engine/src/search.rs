//! Negamax alpha-beta with iterative deepening, MVV-LVA ordering, quiescence,
//! check extensions and repetition / 50-move draws.
//! No I/O in here (WASM-safe): progress goes out through a callback.

use crate::eval::evaluate;
use crate::movegen::{gen_legal, legal_moves, MoveList, MAX_MOVES};
use crate::nnue::{self, Acc};
use crate::position::*;
use crate::see::see_ge;
use crate::tt::{self, Tt};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

pub const MATE: i32 = 30_000;
pub const INF: i32 = 32_000;
pub const MAX_PLY: usize = 128;
const HIST_MAX: i32 = 16_384;
/// NNUE output is clamped so it can never be mistaken for a mate score.
const EVAL_CLAMP: i32 = 10_000;
/// Reverse futility pruning: margin per remaining ply, and the deepest node it applies to.
const RFP_MARGIN: i32 = 100;
const RFP_MAX_DEPTH: i32 = 6;
/// Futility pruning: margin per remaining ply, and the deepest node it applies to.
const FP_MARGIN: i32 = 100;
const FP_MAX_DEPTH: i32 = 4;
/// Aspiration windows: from this depth the root is searched in a narrow window
/// around the previous score, widening on a fail low / fail high.
const ASP_MIN_DEPTH: i32 = 5;
const ASP_DELTA: i32 = 25;

#[inline]
fn hist_idx(color: usize, m: Move) -> usize {
    color * 4096 + m.from as usize * 64 + m.to as usize
}

#[derive(Default, Clone, Debug)]
pub struct Limits {
    pub depth: Option<i32>,
    pub movetime: Option<u64>,
    pub wtime: Option<u64>,
    pub btime: Option<u64>,
    pub winc: Option<u64>,
    pub binc: Option<u64>,
    pub movestogo: Option<u64>,
    /// Soft node limit: stop starting new iterations past this (hard cap at 8x).
    pub nodes: Option<u64>,
}

impl Limits {
    /// Milliseconds to spend on this move, or None for no time limit.
    fn time_for(&self, stm: usize) -> Option<u64> {
        if let Some(mt) = self.movetime {
            return Some(mt.saturating_sub(10).max(1));
        }
        let (t, inc) = if stm == WHITE { (self.wtime, self.winc) } else { (self.btime, self.binc) };
        let t = t?;
        let mtg = self.movestogo.unwrap_or(30).max(1);
        let alloc = t / mtg + inc.unwrap_or(0) * 3 / 4;
        Some(alloc.min(t.saturating_sub(50)).max(1))
    }
}

pub struct Info {
    pub depth: i32,
    pub score: i32,
    pub nodes: u64,
    pub time_ms: u64,
    pub best_move: Move,
}

pub struct SearchResult {
    pub best_move: Option<Move>,
    pub score: i32,
    pub depth: i32,
    pub nodes: u64,
}

pub struct Searcher {
    nodes: u64,
    start: Instant,
    time_limit_ms: Option<u64>,
    node_limit: Option<u64>,
    stop: Arc<AtomicBool>,
    aborted: bool,
    /// Hashes of every position from the game start through the current node.
    history: Vec<u64>,
    root_pv: Option<Move>,
    root_best: Option<Move>,
    tt: Tt,
    /// Two quiet moves per ply that recently caused a beta cutoff.
    killers: [[Option<Move>; 2]; MAX_PLY],
    /// History heuristic: [color][from][to], bounded by gravity updates.
    quiet_hist: Vec<i32>,
    /// Late-move reduction table: [depth][move number], built once.
    lmr: [[i32; 64]; 64],
    /// Use the NNUE net instead of the handcrafted eval.
    nnue: bool,
    /// NNUE accumulators, one per ply (child = parent + move delta).
    accs: Vec<Acc>,
    /// Reverse futility pruning margin per ply (UCI option RfpMargin, for tuning).
    rfp_margin: i32,
    /// Futility pruning margin per ply (UCI option FpMargin, for tuning).
    fp_margin: i32,
    /// Skip losing captures (SEE < 0) in quiescence (UCI option SeeQs).
    see_qs: bool,
    /// Order losing captures after the quiet moves in the main search (UCI option SeeOrder).
    see_order: bool,
}

/// The piece type captured by `m`, if any (handles en passant).
fn captured(pos: &Position, m: Move) -> Option<usize> {
    if let Some(pc) = pos.piece_on(m.to, pos.stm ^ 1) {
        return Some(pc);
    }
    if pos.ep != NO_SQ && m.to == pos.ep && (m.from & 7) != (m.to & 7) && (pos.pieces[PAWN] >> m.from) & 1 == 1 {
        return Some(PAWN);
    }
    None
}

fn move_score(pos: &Position, m: Move) -> i32 {
    let mut s = 0;
    if let Some(victim) = captured(pos, m) {
        let attacker = pos.piece_at(m.from).map(|x| x.1).unwrap_or(0);
        s += 10_000 + victim as i32 * 10 - attacker as i32; // MVV-LVA
    }
    if m.promo != 0 {
        s += 8_000 + m.promo as i32;
    }
    s
}

fn score_moves(pos: &Position, moves: &MoveList, first: Option<Move>, scores: &mut [i32; MAX_MOVES]) {
    for i in 0..moves.len() {
        let m = moves.get(i);
        scores[i] = if Some(m) == first { 1_000_000 } else { move_score(pos, m) };
    }
}

/// Selection step: bring the best-scored remaining move to index `start`.
#[inline]
fn pick_next(moves: &mut MoveList, scores: &mut [i32; MAX_MOVES], start: usize) -> (Move, i32) {
    let mut best = start;
    for i in start + 1..moves.len() {
        if scores[i] > scores[best] {
            best = i;
        }
    }
    moves.swap(start, best);
    scores.swap(start, best);
    (moves.get(start), scores[start])
}

impl Searcher {
    pub fn new(stop: Arc<AtomicBool>, hash_mb: usize) -> Searcher {
        let mut lmr = [[0i32; 64]; 64];
        for d in 1..64 {
            for m in 1..64 {
                lmr[d][m] = (0.75 + (d as f64).ln() * (m as f64).ln() / 2.25) as i32;
            }
        }
        Searcher {
            nodes: 0,
            start: Instant::now(),
            time_limit_ms: None,
            node_limit: None,
            stop,
            aborted: false,
            history: Vec::new(),
            root_pv: None,
            root_best: None,
            tt: Tt::new(hash_mb),
            killers: [[None; 2]; MAX_PLY],
            quiet_hist: vec![0; 2 * 4096],
            lmr,
            nnue: false,
            accs: vec![Acc::default(); MAX_PLY + 2],
            rfp_margin: RFP_MARGIN,
            fp_margin: FP_MARGIN,
            see_qs: true,
            see_order: true,
        }
    }

    pub fn set_nnue(&mut self, on: bool) {
        self.nnue = on;
    }

    pub fn set_rfp_margin(&mut self, margin: i32) {
        self.rfp_margin = margin;
    }

    pub fn set_fp_margin(&mut self, margin: i32) {
        self.fp_margin = margin;
    }

    pub fn set_see_qs(&mut self, on: bool) {
        self.see_qs = on;
    }

    pub fn set_see_order(&mut self, on: bool) {
        self.see_order = on;
    }

    /// Static eval of the node at `ply` (its accumulator must be current).
    #[inline]
    fn eval(&self, pos: &Position, ply: usize) -> i32 {
        if self.nnue {
            debug_assert!(self.accs[ply].vals == Acc::from_position(pos).vals, "stale accumulator at ply {ply}");
            nnue::network().evaluate(&self.accs[ply], pos.stm).clamp(-EVAL_CLAMP, EVAL_CLAMP)
        } else {
            evaluate(pos)
        }
    }

    /// Prepare the accumulator for the child reached by `m` from the node at `ply`.
    #[inline]
    fn push_child(&mut self, pos: &Position, m: Move, ply: usize) {
        if self.nnue {
            let next = self.accs[ply].after_move(pos, m);
            self.accs[ply + 1] = next;
        }
    }

    /// A null move changes no pieces, and accumulators are per colour, so just copy.
    #[inline]
    fn push_null(&mut self, ply: usize) {
        if self.nnue {
            self.accs[ply + 1] = self.accs[ply];
        }
    }

    pub fn resize_tt(&mut self, mb: usize) {
        self.tt = Tt::new(mb);
    }

    pub fn new_game(&mut self) {
        self.tt.clear();
        self.quiet_hist.iter_mut().for_each(|h| *h = 0);
    }

    fn bump_hist(&mut self, color: usize, m: Move, bonus: i32) {
        let h = &mut self.quiet_hist[hist_idx(color, m)];
        *h += bonus - *h * bonus.abs() / HIST_MAX;
    }

    /// Ordering for the main search: TT/PV move, captures (MVV-LVA), promotions,
    /// killers, then quiet moves by history score.
    fn score_moves_main(&self, pos: &Position, moves: &MoveList, first: Option<Move>, ply: usize, scores: &mut [i32; MAX_MOVES]) {
        for i in 0..moves.len() {
            let m = moves.get(i);
            scores[i] = if Some(m) == first {
                1_000_000
            } else {
                let mut s = move_score(pos, m);
                if s != 0 {
                    // A capture (no promotion) that loses material goes after every quiet move.
                    if self.see_order && m.promo == 0 && !see_ge(pos, m, 0) {
                        s -= 30_000;
                    }
                    s
                } else if Some(m) == self.killers[ply][0] {
                    7_000
                } else if Some(m) == self.killers[ply][1] {
                    6_000
                } else {
                    self.quiet_hist[hist_idx(pos.stm, m)].clamp(-5_500, 5_500)
                }
            };
        }
    }

    /// `history` = hashes of all game positions up to and including `pos`.
    pub fn search(
        &mut self,
        pos: &Position,
        history: Vec<u64>,
        limits: &Limits,
        on_info: &mut dyn FnMut(&Info),
    ) -> SearchResult {
        self.nodes = 0;
        self.start = Instant::now();
        self.aborted = false;
        self.history = history;
        self.killers = [[None; 2]; MAX_PLY];
        self.time_limit_ms = limits.time_for(pos.stm);
        self.node_limit = limits.nodes;
        if self.nnue {
            self.accs[0] = Acc::from_position(pos);
        }

        let root_moves = legal_moves(pos);
        let mut best_move = root_moves.first().copied(); // fallback if aborted immediately
        let mut best_score = 0;
        let mut done_depth = 0;
        if root_moves.is_empty() {
            let score = if pos.in_check() { -MATE } else { 0 };
            return SearchResult { best_move: None, score, depth: 0, nodes: 0 };
        }

        let max_depth = limits.depth.unwrap_or(64).clamp(1, (MAX_PLY - 1) as i32);
        for depth in 1..=max_depth {
            self.root_pv = best_move;
            self.root_best = None;
            let mut score;
            if depth >= ASP_MIN_DEPTH {
                let mut delta = ASP_DELTA;
                let mut alpha = (best_score - delta).max(-INF);
                let mut beta = (best_score + delta).min(INF);
                loop {
                    self.root_best = None;
                    score = self.negamax(pos, depth, alpha, beta, 0, true);
                    if self.aborted {
                        break;
                    }
                    if score <= alpha {
                        // Fail low: pull alpha down, and beta toward it.
                        beta = (alpha + beta) / 2;
                        alpha = (score - delta).max(-INF);
                    } else if score >= beta {
                        // Fail high: the move that beat beta is searched first next time.
                        if self.root_best.is_some() {
                            self.root_pv = self.root_best;
                        }
                        beta = (score + delta).min(INF);
                    } else {
                        break;
                    }
                    delta += delta / 2;
                    if delta > 800 {
                        alpha = -INF;
                        beta = INF;
                    }
                }
            } else {
                score = self.negamax(pos, depth, -INF, INF, 0, true);
            }
            if self.aborted {
                break;
            }
            best_move = self.root_best;
            best_score = score;
            done_depth = depth;
            on_info(&Info {
                depth,
                score,
                nodes: self.nodes,
                time_ms: self.start.elapsed().as_millis() as u64,
                best_move: best_move.unwrap(),
            });
            if score.abs() >= MATE - MAX_PLY as i32 {
                break; // forced mate found
            }
            if let Some(n) = self.node_limit {
                if self.nodes >= n {
                    break;
                }
            }
            if let Some(t) = self.time_limit_ms {
                // Unlikely to finish the next iteration in the remaining time.
                if self.start.elapsed().as_millis() as u64 * 2 > t {
                    break;
                }
            }
        }
        SearchResult { best_move, score: best_score, depth: done_depth, nodes: self.nodes }
    }

    fn check_abort(&mut self) {
        if self.aborted || self.nodes & 2047 != 0 {
            return;
        }
        if self.stop.load(Ordering::Relaxed) {
            self.aborted = true;
            return;
        }
        if let Some(n) = self.node_limit {
            if self.nodes >= n.saturating_mul(2) {
                self.aborted = true;
                return;
            }
        }
        if let Some(t) = self.time_limit_ms {
            if self.start.elapsed().as_millis() as u64 >= t {
                self.aborted = true;
            }
        }
    }

    fn is_repetition(&self, pos: &Position) -> bool {
        let len = self.history.len();
        if len < 3 {
            return false;
        }
        let n = (pos.halfmove as usize).min(len - 1);
        let mut i = 2;
        while i <= n {
            if self.history[len - 1 - i] == pos.hash {
                return true;
            }
            i += 2;
        }
        false
    }

    fn negamax(&mut self, pos: &Position, mut depth: i32, mut alpha: i32, beta: i32, ply: usize, allow_null: bool) -> i32 {
        self.nodes += 1;
        self.check_abort();
        if self.aborted {
            return 0;
        }
        if ply > 0 && (pos.halfmove >= 100 || self.is_repetition(pos)) {
            return 0;
        }
        if ply >= MAX_PLY {
            return self.eval(pos, ply);
        }
        let in_check = pos.in_check();
        if in_check {
            depth += 1; // check extension
        }
        if depth <= 0 {
            return self.quiesce(pos, alpha, beta, ply);
        }

        let orig_alpha = alpha;
        let mut tt_move = None;
        if let Some(e) = self.tt.probe(pos.hash) {
            tt_move = e.best_move();
            if ply > 0 && e.depth as i32 >= depth {
                let s = tt::from_tt_score(e.score as i32, ply);
                match e.flag {
                    tt::EXACT => return s,
                    tt::LOWER if s >= beta => return s,
                    tt::UPPER if s <= alpha => return s,
                    _ => {}
                }
            }
        }

        // Static eval of this node, computed once for the pruning below.
        let static_eval = if in_check { -INF } else { self.eval(pos, ply) };

        // Reverse futility pruning: at shallow depth, in a node that is not on the
        // principal variation (zero window), if the static eval is already a safe
        // margin above beta, assume the node fails high and return early. Skipped in
        // check, at the root, and near mate scores.
        if beta - alpha == 1
            && ply > 0
            && !in_check
            && depth <= RFP_MAX_DEPTH
            && beta.abs() < MATE - MAX_PLY as i32
            && static_eval - self.rfp_margin * depth >= beta
        {
            return static_eval - self.rfp_margin * depth;
        }

        // Null-move pruning: if passing still fails high on a reduced search, the
        // real position almost certainly does too. Skipped in check, at the root,
        // in endgames with only pawns (zugzwang risk), near mate scores, and
        // never twice in a row.
        let non_pawn = pos.colors[pos.stm] & !(pos.pieces[PAWN] | pos.pieces[KING]);
        if allow_null
            && ply > 0
            && !in_check
            && depth >= 3
            && non_pawn != 0
            && beta.abs() < MATE - MAX_PLY as i32
            && static_eval >= beta
        {
            let r = 3 + depth / 6;
            let np = pos.make_null_move();
            self.push_null(ply);
            self.history.push(np.hash);
            let score = -self.negamax(&np, depth - 1 - r, -beta, -beta + 1, ply + 1, false);
            self.history.pop();
            if self.aborted {
                return 0;
            }
            if score >= beta {
                return if score >= MATE - MAX_PLY as i32 { beta } else { score };
            }
        }

        let mut moves = MoveList::new();
        gen_legal(pos, &mut moves);
        if moves.is_empty() {
            return if in_check { -MATE + ply as i32 } else { 0 };
        }
        let first = if ply == 0 { self.root_pv } else { tt_move };
        let mut scores = [0i32; MAX_MOVES];
        self.score_moves_main(pos, &moves, first, ply, &mut scores);

        let mut best = -INF;
        let mut best_move: Option<Move> = None;
        let mut quiets = [Move { from: 0, to: 0, promo: 0 }; 64];
        let mut nq = 0usize;
        for i in 0..moves.len() {
            let (m, _) = pick_next(&mut moves, &mut scores, i);
            let is_quiet = captured(pos, m).is_none() && m.promo == 0;
            let child = pos.make_move(m);
            // Futility pruning: at shallow depth in a zero-window node, a quiet move that
            // does not give check is skipped when the static eval plus a margin still
            // cannot reach alpha. Never before the first move has been searched, never in
            // check, at the root, or near mate scores.
            if i > 0
                && ply > 0
                && is_quiet
                && !in_check
                && beta - alpha == 1
                && depth <= FP_MAX_DEPTH
                && alpha.abs() < MATE - MAX_PLY as i32
                && static_eval + self.fp_margin * depth <= alpha
                && !child.in_check()
            {
                continue;
            }
            self.push_child(pos, m, ply);
            self.history.push(child.hash);
            let new_depth = depth - 1;
            let mut score;
            if i == 0 {
                // First move (the expected best): full window.
                score = -self.negamax(&child, new_depth, -beta, -alpha, ply + 1, true);
            } else {
                // Principal-variation search: every later move is expected to fail low,
                // so first test it with a zero window, which is much cheaper. On top of
                // that, late-move reductions: moves late in the ordering are probably
                // bad, so look at them shallower too. Never reduce in check, captures,
                // promotions, or moves that give check.
                let r = if i >= 3 && depth >= 3 && is_quiet && !in_check && !child.in_check() {
                    self.lmr[(depth as usize).min(63)][i.min(63)].clamp(0, new_depth - 1)
                } else {
                    0
                };
                score = -self.negamax(&child, new_depth - r, -alpha - 1, -alpha, ply + 1, true);
                // A reduced move that beat alpha: confirm at full depth, still zero window.
                if score > alpha && r > 0 {
                    score = -self.negamax(&child, new_depth, -alpha - 1, -alpha, ply + 1, true);
                }
                // It really is better than the first move (and not a cut-off): re-search
                // with the full window to get its exact score.
                if score > alpha && score < beta {
                    score = -self.negamax(&child, new_depth, -beta, -alpha, ply + 1, true);
                }
            }
            self.history.pop();
            if self.aborted {
                return 0;
            }
            if score > best {
                best = score;
                best_move = Some(m);
                if ply == 0 {
                    self.root_best = Some(m);
                }
                if score > alpha {
                    alpha = score;
                    if alpha >= beta {
                        if is_quiet {
                            if self.killers[ply][0] != Some(m) {
                                self.killers[ply][1] = self.killers[ply][0];
                                self.killers[ply][0] = Some(m);
                            }
                            let bonus = (depth * depth).min(1600);
                            self.bump_hist(pos.stm, m, bonus);
                            for k in 0..nq {
                                self.bump_hist(pos.stm, quiets[k], -bonus);
                            }
                        }
                        break;
                    }
                }
            }
            if is_quiet && nq < 64 {
                quiets[nq] = m;
                nq += 1;
            }
        }
        let flag = if best <= orig_alpha {
            tt::UPPER
        } else if best >= beta {
            tt::LOWER
        } else {
            tt::EXACT
        };
        self.tt.store(pos.hash, best_move, tt::to_tt_score(best, ply), depth, flag);
        best
    }

    fn quiesce(&mut self, pos: &Position, mut alpha: i32, beta: i32, ply: usize) -> i32 {
        self.nodes += 1;
        self.check_abort();
        if self.aborted {
            return 0;
        }
        let stand = self.eval(pos, ply);
        if ply >= MAX_PLY || stand >= beta {
            return stand;
        }
        if stand > alpha {
            alpha = stand;
        }
        let mut moves = MoveList::new();
        gen_legal(pos, &mut moves);
        let mut scores = [0i32; MAX_MOVES];
        score_moves(pos, &moves, None, &mut scores);

        let mut best = stand;
        for i in 0..moves.len() {
            let (m, sc) = pick_next(&mut moves, &mut scores, i);
            if sc == 0 {
                break; // only quiet moves left (captures and promotions score > 0)
            }
            if self.see_qs && m.promo == 0 && !see_ge(pos, m, 0) {
                continue; // losing capture: not worth searching in quiescence
            }
            let child = pos.make_move(m);
            self.push_child(pos, m, ply);
            let score = -self.quiesce(&child, -beta, -alpha, ply + 1);
            if self.aborted {
                return 0;
            }
            if score > best {
                best = score;
                if score > alpha {
                    alpha = score;
                    if alpha >= beta {
                        break;
                    }
                }
            }
        }
        best
    }
}
