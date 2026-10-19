//! Negamax alpha-beta with iterative deepening, MVV-LVA ordering, quiescence,
//! check extensions and repetition / 50-move draws.
//! No I/O in here (WASM-safe): progress goes out through a callback.

use crate::eval::evaluate;
use crate::movegen::{gen_legal, legal_moves, MoveList, MAX_MOVES};
use crate::position::*;
use crate::tt::{self, Tt};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

pub const MATE: i32 = 30_000;
pub const INF: i32 = 32_000;
pub const MAX_PLY: usize = 128;

#[derive(Default, Clone, Debug)]
pub struct Limits {
    pub depth: Option<i32>,
    pub movetime: Option<u64>,
    pub wtime: Option<u64>,
    pub btime: Option<u64>,
    pub winc: Option<u64>,
    pub binc: Option<u64>,
    pub movestogo: Option<u64>,
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
    stop: Arc<AtomicBool>,
    aborted: bool,
    /// Hashes of every position from the game start through the current node.
    history: Vec<u64>,
    root_pv: Option<Move>,
    root_best: Option<Move>,
    tt: Tt,
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
        Searcher {
            nodes: 0,
            start: Instant::now(),
            time_limit_ms: None,
            stop,
            aborted: false,
            history: Vec::new(),
            root_pv: None,
            root_best: None,
            tt: Tt::new(hash_mb),
        }
    }

    pub fn resize_tt(&mut self, mb: usize) {
        self.tt = Tt::new(mb);
    }

    pub fn new_game(&mut self) {
        self.tt.clear();
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
        self.time_limit_ms = limits.time_for(pos.stm);

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
            let score = self.negamax(pos, depth, -INF, INF, 0);
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

    fn negamax(&mut self, pos: &Position, mut depth: i32, mut alpha: i32, beta: i32, ply: usize) -> i32 {
        self.nodes += 1;
        self.check_abort();
        if self.aborted {
            return 0;
        }
        if ply > 0 && (pos.halfmove >= 100 || self.is_repetition(pos)) {
            return 0;
        }
        if ply >= MAX_PLY {
            return evaluate(pos);
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

        let mut moves = MoveList::new();
        gen_legal(pos, &mut moves);
        if moves.is_empty() {
            return if in_check { -MATE + ply as i32 } else { 0 };
        }
        let first = if ply == 0 { self.root_pv } else { tt_move };
        let mut scores = [0i32; MAX_MOVES];
        score_moves(pos, &moves, first, &mut scores);

        let mut best = -INF;
        let mut best_move: Option<Move> = None;
        for i in 0..moves.len() {
            let (m, _) = pick_next(&mut moves, &mut scores, i);
            let child = pos.make_move(m);
            self.history.push(child.hash);
            let score = -self.negamax(&child, depth - 1, -beta, -alpha, ply + 1);
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
                        break;
                    }
                }
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
        let stand = evaluate(pos);
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
            let child = pos.make_move(m);
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
