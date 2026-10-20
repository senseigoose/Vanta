use engine::{legal_moves, Position};

fn walk(pos: &Position, depth: u32) {
    assert_eq!(pos.hash, pos.compute_hash(), "incremental hash drifted");
    for sq in 0..64u8 {
        let mut expected = None;
        for c in 0..2 {
            for pc in 0..6 {
                if (pos.pieces[pc] & pos.colors[c]) >> sq & 1 == 1 {
                    expected = Some((c, pc));
                }
            }
        }
        assert_eq!(pos.piece_at(sq), expected, "mailbox out of sync on square {sq}");
    }
    if !pos.in_check() {
        let np = pos.make_null_move();
        assert_eq!(np.hash, np.compute_hash(), "null-move hash drifted");
    }
    if depth == 0 {
        return;
    }
    for m in legal_moves(pos) {
        walk(&pos.make_move(m), depth - 1);
    }
}

#[test]
fn incremental_hash_matches_recompute() {
    for fen in [
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
        "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
        "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
        "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
    ] {
        walk(&Position::from_fen(fen).unwrap(), 3);
    }
}

#[test]
fn transposition_gives_same_hash() {
    // 1.Nf3 Nf6 2.Nc3 Nc6 reached by two move orders.
    let play = |moves: &[&str]| {
        let mut p = Position::startpos();
        for &u in moves {
            let m = legal_moves(&p).into_iter().find(|m| m.to_uci() == u).expect(u);
            p = p.make_move(m);
        }
        p.hash
    };
    let a = play(&["g1f3", "g8f6", "b1c3", "b8c6"]);
    let b = play(&["b1c3", "b8c6", "g1f3", "g8f6"]);
    assert_eq!(a, b);
}
