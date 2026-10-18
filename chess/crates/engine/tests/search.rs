use engine::search::{Info, Limits, Searcher, MATE};
use engine::Position;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

fn best(fen: &str, depth: i32) -> (String, i32) {
    let pos = Position::from_fen(fen).unwrap();
    let mut s = Searcher::new(Arc::new(AtomicBool::new(false)), 1);
    let limits = Limits { depth: Some(depth), ..Default::default() };
    let r = s.search(&pos, vec![pos.hash], &limits, &mut |_i: &Info| {});
    (r.best_move.unwrap().to_uci(), r.score)
}

#[test]
fn finds_mate_in_one() {
    let (mv, score) = best("6k1/5ppp/8/8/8/8/8/R5K1 w - - 0 1", 3);
    assert_eq!(mv, "a1a8");
    assert!(score > MATE - 10, "score {score}");
}

#[test]
fn takes_free_queen() {
    let (mv, _) = best("4k3/8/8/3q4/8/8/8/3QK3 w - - 0 1", 3);
    assert_eq!(mv, "d1d5");
}

#[test]
fn startpos_returns_a_move() {
    let (mv, score) = best("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", 5);
    assert!(!mv.is_empty());
    assert!(score.abs() < 100, "startpos should be roughly equal, got {score}");
}
