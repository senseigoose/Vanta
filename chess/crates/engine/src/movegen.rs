use crate::bitboard::*;
use crate::position::*;

fn add_pawn_move(out: &mut Vec<Move>, from: u8, to: u8, promo_rank: u8) {
    if to >> 3 == promo_rank {
        for promo in [KNIGHT, BISHOP, ROOK, QUEEN] {
            out.push(Move { from, to, promo: promo as u8 });
        }
    } else {
        out.push(Move { from, to, promo: 0 });
    }
}

/// Pseudo-legal moves (may leave own king in check).
pub fn pseudo_legal(pos: &Position, out: &mut Vec<Move>) {
    let us = pos.stm;
    let them = us ^ 1;
    let occ = pos.occupied();
    let own = pos.colors[us];
    let enemy = pos.colors[them];

    // Pawns
    let (push, start_rank, promo_rank): (i8, u8, u8) = if us == WHITE { (8, 1, 7) } else { (-8, 6, 0) };
    for from in bits(pos.pieces[PAWN] & own) {
        let one = (from as i8 + push) as u8;
        if (occ >> one) & 1 == 0 {
            add_pawn_move(out, from, one, promo_rank);
            if from >> 3 == start_rank {
                let two = (from as i8 + 2 * push) as u8;
                if (occ >> two) & 1 == 0 {
                    out.push(Move { from, to: two, promo: 0 });
                }
            }
        }
        let caps = pawn_attacks(us, from);
        for to in bits(caps & enemy) {
            add_pawn_move(out, from, to, promo_rank);
        }
        if pos.ep != NO_SQ && (caps >> pos.ep) & 1 == 1 {
            out.push(Move { from, to: pos.ep, promo: 0 });
        }
    }

    // Knights, bishops, rooks, queens, king
    let mut gen_piece = |piece: usize, f: &dyn Fn(u8) -> Bb| {
        for from in bits(pos.pieces[piece] & own) {
            for to in bits(f(from) & !own) {
                out.push(Move { from, to, promo: 0 });
            }
        }
    };
    gen_piece(KNIGHT, &|s| knight_attacks(s));
    gen_piece(BISHOP, &|s| bishop_attacks(s, occ));
    gen_piece(ROOK, &|s| rook_attacks(s, occ));
    gen_piece(QUEEN, &|s| bishop_attacks(s, occ) | rook_attacks(s, occ));
    gen_piece(KING, &|s| king_attacks(s));

    // Castling. The destination square is checked by the legality filter;
    // here we check emptiness, not-in-check, and the pass-through square.
    let empty = |sqs: &[u8]| sqs.iter().all(|&s| (occ >> s) & 1 == 0);
    if us == WHITE {
        if pos.castling & WK != 0 && empty(&[5, 6]) && !pos.is_attacked(4, BLACK) && !pos.is_attacked(5, BLACK) {
            out.push(Move { from: 4, to: 6, promo: 0 });
        }
        if pos.castling & WQ != 0 && empty(&[1, 2, 3]) && !pos.is_attacked(4, BLACK) && !pos.is_attacked(3, BLACK) {
            out.push(Move { from: 4, to: 2, promo: 0 });
        }
    } else {
        if pos.castling & BK != 0 && empty(&[61, 62]) && !pos.is_attacked(60, WHITE) && !pos.is_attacked(61, WHITE) {
            out.push(Move { from: 60, to: 62, promo: 0 });
        }
        if pos.castling & BQ != 0 && empty(&[57, 58, 59]) && !pos.is_attacked(60, WHITE) && !pos.is_attacked(59, WHITE) {
            out.push(Move { from: 60, to: 58, promo: 0 });
        }
    }
}

pub fn legal_moves(pos: &Position) -> Vec<Move> {
    let mut pseudo = Vec::with_capacity(64);
    pseudo_legal(pos, &mut pseudo);
    let us = pos.stm;
    pseudo
        .into_iter()
        .filter(|&m| {
            let child = pos.make_move(m);
            !child.is_attacked(child.king_sq(us), us ^ 1)
        })
        .collect()
}

pub fn perft(pos: &Position, depth: u32) -> u64 {
    if depth == 0 {
        return 1;
    }
    let moves = legal_moves(pos);
    if depth == 1 {
        return moves.len() as u64;
    }
    moves.iter().map(|&m| perft(&pos.make_move(m), depth - 1)).sum()
}
