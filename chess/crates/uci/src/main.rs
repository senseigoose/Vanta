use engine::position::{Position, STARTPOS};
use engine::search::{Info, Limits, Searcher, MATE, MAX_PLY};
use engine::{legal_moves, perft, Move};
use std::io::{self, BufRead};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 3 && args[1] == "perft" {
        run_perft(&args);
    } else {
        uci_loop();
    }
}

// Usage: uci perft <depth> [fen]   (prints per-move "divide" counts)
fn run_perft(args: &[String]) {
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

fn fmt_score(s: i32) -> String {
    if s.abs() >= MATE - MAX_PLY as i32 {
        let n = (MATE - s.abs() + 1) / 2;
        format!("mate {}", if s > 0 { n } else { -n })
    } else {
        format!("cp {}", s)
    }
}

fn find_move(pos: &Position, uci: &str) -> Option<Move> {
    legal_moves(pos).into_iter().find(|m| m.to_uci() == uci)
}

/// Returns the position and the hash history (game start .. current).
fn parse_position(line: &str) -> Option<(Position, Vec<u64>)> {
    let t: Vec<&str> = line.split_whitespace().collect();
    let (mut pos, mut i) = match t.get(1) {
        Some(&"startpos") => (Position::startpos(), 2),
        Some(&"fen") => {
            let end = t.iter().position(|&x| x == "moves").unwrap_or(t.len());
            (Position::from_fen(&t[2..end].join(" ")).ok()?, end)
        }
        _ => return None,
    };
    let mut history = vec![pos.hash];
    if t.get(i) == Some(&"moves") {
        i += 1;
        for u in &t[i..] {
            match find_move(&pos, u) {
                Some(m) => {
                    pos = pos.make_move(m);
                    history.push(pos.hash);
                }
                None => break,
            }
        }
    }
    Some((pos, history))
}

fn parse_go(line: &str) -> Limits {
    let t: Vec<&str> = line.split_whitespace().collect();
    let mut l = Limits::default();
    let num = |i: usize| t.get(i + 1).and_then(|s| s.parse::<u64>().ok());
    let mut i = 1;
    while i < t.len() {
        match t[i] {
            "depth" => l.depth = num(i).map(|v| v as i32),
            "movetime" => l.movetime = num(i),
            "wtime" => l.wtime = num(i),
            "btime" => l.btime = num(i),
            "winc" => l.winc = num(i),
            "binc" => l.binc = num(i),
            "movestogo" => l.movestogo = num(i),
            _ => {}
        }
        i += 1;
    }
    l
}

fn uci_loop() {
    let stop = Arc::new(AtomicBool::new(false));
    let (tx, rx) = mpsc::channel::<String>();

    // Reader thread: lets "stop" interrupt a running search. It resets the flag
    // on "go" *before* forwarding, so a stop sent right after go is never lost.
    {
        let stop = stop.clone();
        thread::spawn(move || {
            for line in io::stdin().lock().lines() {
                let Ok(line) = line else { break };
                let cmd = line.split_whitespace().next().unwrap_or("");
                match cmd {
                    "go" => stop.store(false, Ordering::SeqCst),
                    "stop" | "quit" => stop.store(true, Ordering::SeqCst),
                    _ => {}
                }
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
    }

    let mut pos = Position::startpos();
    let mut history = vec![pos.hash];

    while let Ok(line) = rx.recv() {
        let line = line.trim().to_string();
        match line.split_whitespace().next().unwrap_or("") {
            "uci" => {
                println!("id name Vanta 0.1");
                println!("id author senseigoose");
                println!("uciok");
            }
            "isready" => println!("readyok"),
            "ucinewgame" => {
                pos = Position::startpos();
                history = vec![pos.hash];
            }
            "position" => {
                if let Some((p, h)) = parse_position(&line) {
                    pos = p;
                    history = h;
                }
            }
            "go" => {
                let limits = parse_go(&line);
                let mut searcher = Searcher::new(stop.clone());
                let res = searcher.search(&pos, history.clone(), &limits, &mut |i: &Info| {
                    println!(
                        "info depth {} score {} nodes {} time {} nps {} pv {}",
                        i.depth,
                        fmt_score(i.score),
                        i.nodes,
                        i.time_ms,
                        i.nodes * 1000 / i.time_ms.max(1),
                        i.best_move.to_uci()
                    );
                });
                match res.best_move {
                    Some(m) => println!("bestmove {}", m.to_uci()),
                    None => println!("bestmove 0000"),
                }
            }
            "quit" => break,
            _ => {}
        }
    }
}
