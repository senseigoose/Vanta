use engine::position::{Position, STARTPOS};
use engine::search::{Info, Limits, Searcher, MATE, MAX_PLY};
use engine::{legal_moves, perft, Move};
use std::io::{self, BufRead};
use std::sync::atomic::{AtomicBool, Ordering};
use std::collections::HashSet;
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 3 && args[1] == "perft" {
        run_perft(&args);
    } else if args.len() >= 3 && args[1] == "evalfen" {
        run_evalfen(&args);
    } else if args.len() >= 2 && args[1] == "bench" {
        run_bench(&args);
    } else if args.len() >= 2 && args[1] == "genbook" {
        run_genbook(&args);
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

// Usage: uci evalfen <fen>   (prints the handcrafted and NNUE static evals, side-to-move relative)
fn run_evalfen(args: &[String]) {
    let fen = args[2..].join(" ");
    let pos = Position::from_fen(&fen).expect("bad FEN");
    println!("hce {}", engine::eval::evaluate(&pos));
    println!("nnue {}", engine::nnue::evaluate_position(&pos));
}

// Usage: uci bench [nnue]   (fixed-depth searches on six positions; prints nodes and nps)
fn run_bench(args: &[String]) {
    let fens = [
        STARTPOS,
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
        "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
        "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
        "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
    ];
    let mut searcher = Searcher::new(Arc::new(AtomicBool::new(false)), 16);
    searcher.set_nnue(args.get(2).map(|s| s == "nnue") == Some(true));
    let limits = Limits { depth: Some(6), ..Default::default() };
    let start = Instant::now();
    let mut nodes = 0u64;
    for fen in fens {
        let pos = Position::from_fen(fen).unwrap();
        searcher.new_game();
        let r = searcher.search(&pos, vec![pos.hash], &limits, &mut |_i: &Info| {});
        nodes += r.nodes;
    }
    let ms = start.elapsed().as_millis().max(1) as u64;
    println!("nodes {}  time {}ms  nps {}", nodes, ms, nodes * 1000 / ms);
}

fn next_rand(x: &mut u64) -> u64 {
    *x ^= *x << 13;
    *x ^= *x >> 7;
    *x ^= *x << 17;
    *x
}

// Usage: uci genbook [count] [plies] [seed] > book.epd
// Random openings, kept only if a depth-4 search says they are roughly balanced.
fn run_genbook(args: &[String]) {
    let arg = |i: usize, d: u64| args.get(i).and_then(|s| s.parse().ok()).unwrap_or(d);
    let (count, plies) = (arg(2, 1000) as usize, arg(3, 8) as usize);
    let mut rng = arg(4, 1).wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
    let mut searcher = Searcher::new(Arc::new(AtomicBool::new(false)), 16);
    let mut seen = HashSet::new();
    let mut out = 0;
    while out < count {
        let mut pos = Position::startpos();
        let mut ok = true;
        for _ in 0..plies {
            let moves = legal_moves(&pos);
            if moves.is_empty() {
                ok = false;
                break;
            }
            pos = pos.make_move(moves[(next_rand(&mut rng) % moves.len() as u64) as usize]);
        }
        if !ok || legal_moves(&pos).is_empty() {
            continue;
        }
        let epd = pos.to_epd();
        if !seen.insert(epd.clone()) {
            continue;
        }
        searcher.new_game();
        let limits = Limits { depth: Some(4), ..Default::default() };
        let r = searcher.search(&pos, vec![pos.hash], &limits, &mut |_i: &Info| {});
        if r.score.abs() > 100 {
            continue;
        }
        println!("{}", epd);
        out += 1;
    }
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
            "nodes" => l.nodes = num(i),
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
    let mut searcher = Searcher::new(stop.clone(), 16);
    searcher.set_nnue(true);

    while let Ok(line) = rx.recv() {
        let line = line.trim().to_string();
        match line.split_whitespace().next().unwrap_or("") {
            "uci" => {
                println!("id name Vanta {}", env!("CARGO_PKG_VERSION"));
                println!("id author senseigoose");
                println!("option name Hash type spin default 16 min 1 max 1024");
                println!("option name UseNNUE type check default true");
                println!("option name RfpMargin type spin default 100 min 0 max 400");
                println!("option name FpMargin type spin default 100 min 0 max 400");
                println!("option name SeeQs type check default false");
                println!("option name SeeOrder type check default false");
                println!("uciok");
            }
            "isready" => println!("readyok"),
            "setoption" => {
                // setoption name Hash value <mb>
                let t: Vec<&str> = line.split_whitespace().collect();
                if t.get(2).map(|s| s.eq_ignore_ascii_case("usennue")) == Some(true) {
                    if let Some(v) = t.get(4) {
                        searcher.set_nnue(v.eq_ignore_ascii_case("true"));
                    }
                }
                if t.get(2).map(|s| s.eq_ignore_ascii_case("rfpmargin")) == Some(true) {
                    if let Some(m) = t.get(4).and_then(|v| v.parse::<i32>().ok()) {
                        searcher.set_rfp_margin(m.clamp(0, 400));
                    }
                }
                if t.get(2).map(|s| s.eq_ignore_ascii_case("fpmargin")) == Some(true) {
                    if let Some(m) = t.get(4).and_then(|v| v.parse::<i32>().ok()) {
                        searcher.set_fp_margin(m.clamp(0, 400));
                    }
                }
                if t.get(2).map(|s| s.eq_ignore_ascii_case("seeqs")) == Some(true) {
                    if let Some(v) = t.get(4) {
                        searcher.set_see_qs(v.eq_ignore_ascii_case("true"));
                    }
                }
                if t.get(2).map(|s| s.eq_ignore_ascii_case("seeorder")) == Some(true) {
                    if let Some(v) = t.get(4) {
                        searcher.set_see_order(v.eq_ignore_ascii_case("true"));
                    }
                }
                if t.get(2).map(|s| s.eq_ignore_ascii_case("hash")) == Some(true) {
                    if let Some(mb) = t.get(4).and_then(|s| s.parse::<usize>().ok()) {
                        searcher.resize_tt(mb.clamp(1, 1024));
                    }
                }
            }
            "ucinewgame" => {
                searcher.new_game();
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
