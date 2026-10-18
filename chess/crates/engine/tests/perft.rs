use engine::{perft, Position};

fn check(name: &str, fen: &str, expected: &[u64]) {
    let pos = Position::from_fen(fen).unwrap();
    for (i, &want) in expected.iter().enumerate() {
        let depth = (i + 1) as u32;
        assert_eq!(perft(&pos, depth), want, "{name} depth {depth}");
    }
}

#[test]
fn startpos() {
    check("startpos", "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
          &[20, 400, 8902, 197281, 4865609]);
}

#[test]
fn kiwipete() {
    check("kiwipete", "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
          &[48, 2039, 97862, 4085603]);
}

#[test]
fn position3_endgame_ep() {
    check("pos3", "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
          &[14, 191, 2812, 43238, 674624]);
}

#[test]
fn position4_promotions() {
    check("pos4", "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
          &[6, 264, 9467, 422333]);
}

#[test]
fn position5() {
    check("pos5", "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
          &[44, 1486, 62379, 2103487]);
}

#[test]
fn position6() {
    check("pos6", "r4rk1/1pp1qppp/p1np1n2/2b1pb2/2B1P3/2NP1N2/PPP1QPPP/R4RK1 w - - 0 10",
          &[46, 2079, 89890, 3894594]);
}
