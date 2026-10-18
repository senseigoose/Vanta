use engine::position::{Position, STARTPOS};
use engine::{legal_moves, perft};
use std::time::Instant;

// Usage: chess_engine perft <depth> [fen...]   (prints per-move "divide" counts)
fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 || args[1] != "perft" {
        eprintln!("usage: chess_engine perft <depth> [fen]");
        return;
    }
    let depth: u32 = args[2].parse().expect("depth must be a number");
    let fen = if args.len() > 3 { args[3..].join(" ") } else { STARTPOS.to_string() };
    let pos = Position::from_fen(&fen).expect("bad FEN");

    let start = Instant::now();
    let mut total = 0u64;
    for m in legal_moves(&pos) {
        let n = if depth <= 1 { 1 } else { perft(&pos.make_move(m), depth - 1) };
        println!("{}: {}", m.to_uci(), n);
        total += n;
    }
    let secs = start.elapsed().as_secs_f64();
    println!("\nnodes {}  time {:.2}s  nps {:.0}", total, secs, total as f64 / secs.max(1e-9));
}
