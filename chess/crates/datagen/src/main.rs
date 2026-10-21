//! Self-play data generator. Plays fast, low-node games from random openings and
//! writes one Bullet `ChessBoard` record (32 bytes) per quiet position:
//! position + search score + game result.
//!
//!   datagen <out_dir> [threads] [positions] [nodes] [seed] [nnue]
//!
//! nnue: 0 = play and label with the handcrafted eval (default), 1 = use the NNUE net
//!
//! Each thread writes its own file: <out_dir>/data_s<seed>_t<thread>.bin

mod format;

use engine::position::{Position, KNIGHT, BISHOP, PAWN, QUEEN, ROOK, WHITE};
use engine::search::{Info, Limits, Searcher, MATE, MAX_PLY};
use engine::legal_moves;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

const OPENING_PLIES: u64 = 8; // plus 0 or 1 random extra, so either side can be to move
const OPENING_MAX_CP: i32 = 200; // reject unbalanced random openings
const ADJ_CP: i32 = 1000; // adjudicate a win after ADJ_PLIES plies at/above this score
const ADJ_PLIES: u32 = 6;
const MAX_GAME_PLIES: usize = 400;
const MAX_RECORD_CP: i32 = 3000; // don't train on hopeless positions

struct Cfg {
    nodes: u64,
    target: u64,
    nnue: bool,
}

struct Stats {
    positions: AtomicU64,
    games: AtomicU64,
    /// Game-end reasons: checkmate, stalemate, 50-move, repetition, insufficient, max plies, adjudicated.
    ends: [AtomicU64; 7],
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Rng {
        let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        Rng((z ^ (z >> 31)) | 1)
    }
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
}

fn insufficient_material(pos: &Position) -> bool {
    if pos.pieces[PAWN] | pos.pieces[ROOK] | pos.pieces[QUEEN] != 0 {
        return false;
    }
    (pos.pieces[KNIGHT] | pos.pieces[BISHOP]).count_ones() <= 1
}

/// Returns the recorded (position, score from side to move's view) pairs and the
/// game result from White's view (0 loss, 1 draw, 2 win), or None if the opening
/// was rejected.
fn play_game(searcher: &mut Searcher, rng: &mut Rng, cfg: &Cfg) -> Option<(Vec<(Position, i16)>, u8, usize)> {
    searcher.new_game();
    let mut pos = Position::startpos();
    let mut hist = vec![pos.hash];

    for _ in 0..(OPENING_PLIES + rng.next() % 2) {
        let moves = legal_moves(&pos);
        if moves.is_empty() {
            return None;
        }
        pos = pos.make_move(moves[(rng.next() % moves.len() as u64) as usize]);
        hist.push(pos.hash);
    }
    if legal_moves(&pos).is_empty() {
        return None;
    }
    let check = Limits { depth: Some(4), ..Default::default() };
    let r = searcher.search(&pos, hist.clone(), &check, &mut |_i: &Info| {});
    if r.score.abs() > OPENING_MAX_CP {
        return None;
    }

    let limits = Limits { depth: Some(32), nodes: Some(cfg.nodes), ..Default::default() };
    let mut recs: Vec<(Position, i16)> = Vec::new();
    let mut streak = 0u32;
    let mut streak_sign = 0i32;
    let result: u8;
    let reason: usize;

    loop {
        if legal_moves(&pos).is_empty() {
            result = if pos.in_check() {
                reason = 0;
                if pos.stm == WHITE { 0 } else { 2 }
            } else {
                reason = 1;
                1
            };
            break;
        }
        let repeats = hist.iter().filter(|&&h| h == pos.hash).count();
        if pos.halfmove >= 100 || repeats >= 3 || insufficient_material(&pos) || hist.len() > MAX_GAME_PLIES {
            reason = if pos.halfmove >= 100 {
                2
            } else if repeats >= 3 {
                3
            } else if insufficient_material(&pos) {
                4
            } else {
                5
            };
            result = 1;
            break;
        }

        let r = searcher.search(&pos, hist.clone(), &limits, &mut |_i: &Info| {});
        let best = r.best_move.expect("legal moves exist");
        let score = r.score; // side to move's view

        let white_score = if pos.stm == WHITE { score } else { -score };
        if white_score.abs() >= ADJ_CP {
            let sign = white_score.signum();
            streak = if sign == streak_sign { streak + 1 } else { 1 };
            streak_sign = sign;
        } else {
            streak = 0;
            streak_sign = 0;
        }
        if streak >= ADJ_PLIES {
            result = if white_score > 0 { 2 } else { 0 };
            reason = 6;
            break;
        }

        let child = pos.make_move(best);
        let is_capture = child.occupied().count_ones() < pos.occupied().count_ones();
        let quiet = !pos.in_check()
            && !is_capture
            && best.promo == 0
            && !child.in_check()
            && score.abs() < MATE - MAX_PLY as i32
            && score.abs() <= MAX_RECORD_CP;
        if quiet {
            recs.push((pos, score as i16));
        }
        pos = child;
        hist.push(pos.hash);
    }
    Some((recs, result, reason))
}

fn worker(id: u64, seed: u64, cfg: Arc<Cfg>, stats: Arc<Stats>, path: PathBuf) {
    let mut rng = Rng::new(seed.wrapping_mul(1_000_003).wrapping_add(id));
    let mut searcher = Searcher::new(Arc::new(AtomicBool::new(false)), 16);
    searcher.set_nnue(cfg.nnue);
    let mut out = BufWriter::with_capacity(1 << 20, File::create(&path).expect("cannot create output file"));
    while stats.positions.load(Ordering::Relaxed) < cfg.target {
        if let Some((recs, result, reason)) = play_game(&mut searcher, &mut rng, &cfg) {
            stats.ends[reason].fetch_add(1, Ordering::Relaxed);
            for (pos, score) in &recs {
                let r = if pos.stm == WHITE { result } else { 2 - result };
                out.write_all(&format::pack(pos, *score, r)).expect("write failed");
            }
            stats.positions.fetch_add(recs.len() as u64, Ordering::Relaxed);
            stats.games.fetch_add(1, Ordering::Relaxed);
        }
    }
    out.flush().expect("flush failed");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: datagen <out_dir> [threads] [positions] [nodes] [seed] [nnue]");
        return;
    }
    let out_dir = PathBuf::from(&args[1]);
    fs::create_dir_all(&out_dir).expect("cannot create output dir");
    let arg = |i: usize, d: u64| args.get(i).and_then(|s| s.parse().ok()).unwrap_or(d);
    let default_threads = thread::available_parallelism().map(|n| n.get() as u64).unwrap_or(1);
    let threads = arg(2, default_threads).max(1);
    let target = arg(3, 1_000_000);
    let nodes = arg(4, 5000);
    let seed = arg(5, 1);
    let nnue = arg(6, 0) != 0;

    let cfg = Arc::new(Cfg { nodes, target, nnue });
    let stats = Arc::new(Stats {
        positions: AtomicU64::new(0),
        games: AtomicU64::new(0),
        ends: std::array::from_fn(|_| AtomicU64::new(0)),
    });
    println!(
        "datagen: {threads} threads, target {target} positions, {nodes} nodes/move, seed {seed}, eval {}",
        if nnue { "nnue" } else { "hce" }
    );

    let start = Instant::now();
    let handles: Vec<_> = (0..threads)
        .map(|id| {
            let (cfg, stats) = (cfg.clone(), stats.clone());
            let path = out_dir.join(format!("data_s{seed}_t{id}.bin"));
            thread::spawn(move || worker(id, seed, cfg, stats, path))
        })
        .collect();

    while !handles.iter().all(|h| h.is_finished()) {
        thread::sleep(Duration::from_secs(10));
        let p = stats.positions.load(Ordering::Relaxed);
        let g = stats.games.load(Ordering::Relaxed);
        let secs = start.elapsed().as_secs_f64().max(1e-9);
        println!("{p} positions, {g} games, {:.0} pos/s, {:.0}s elapsed", p as f64 / secs, secs);
    }
    for h in handles {
        h.join().expect("worker panicked");
    }
    let p = stats.positions.load(Ordering::Relaxed);
    println!("done: {p} positions ({} bytes) in {:.0}s", p * 32, start.elapsed().as_secs_f64());
    let names = ["checkmate", "stalemate", "50-move", "repetition", "insufficient", "max-plies", "adjudicated"];
    let games = stats.games.load(Ordering::Relaxed).max(1);
    let ends: Vec<String> = names
        .iter()
        .zip(stats.ends.iter())
        .map(|(n, c)| format!("{n} {:.1}%", 100.0 * c.load(Ordering::Relaxed) as f64 / games as f64))
        .collect();
    println!("game ends: {}", ends.join(", "));
}
