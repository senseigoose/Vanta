use crate::bitboard::*;
use crate::position::*;

pub const MAX_MOVES: usize = 320;

/// Fixed-size move buffer on the stack (no heap allocation in the hot path).
#[derive(Clone)]
pub struct MoveList {
    moves: [Move; MAX_MOVES],
    len: usize,
}

impl Default for MoveList {
    fn default() -> Self {
        Self::new()
    }
}

impl MoveList {
    #[inline]
    pub fn new() -> MoveList {
        MoveList { moves: [Move { from: 0, to: 0, promo: 0 }; MAX_MOVES], len: 0 }
    }
    #[inline]
    pub fn push(&mut self, m: Move) {
        self.moves[self.len] = m;
        self.len += 1;
    }
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    #[inline]
    pub fn get(&self, i: usize) -> Move {
        self.moves[i]
    }
    #[inline]
    pub fn swap(&mut self, i: usize, j: usize) {
        self.moves.swap(i, j);
    }
    pub fn as_slice(&self) -> &[Move] {
        &self.moves[..self.len]
    }
}

fn add_pawn_move(out: &mut MoveList, from: u8, to: u8, promo_rank: u8) {
    if to >> 3 == promo_rank {
        for promo in [KNIGHT, BISHOP, ROOK, QUEEN] {
            out.push(Move { from, to, promo: promo as u8 });
        }
    } else {
        out.push(Move { from, to, promo: 0 });
    }
}

/// Pseudo-legal moves (may leave own king in check).
pub fn pseudo_legal(pos: &Position, out: &mut MoveList) {
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

/// Legal moves into `out` (cleared first). Filters in place, no extra buffer.
pub fn gen_legal(pos: &Position, out: &mut MoveList) {
    out.len = 0;
    pseudo_legal(pos, out);
    let us = pos.stm;
    let mut j = 0;
    for i in 0..out.len {
        let m = out.moves[i];
        let child = pos.make_move(m);
        if !child.is_attacked(child.king_sq(us), us ^ 1) {
            out.moves[j] = m;
            j += 1;
        }
    }
    out.len = j;
}

/// Convenience wrapper for non-hot paths (UCI parsing, tests, book generation).
pub fn legal_moves(pos: &Position) -> Vec<Move> {
    let mut list = MoveList::new();
    gen_legal(pos, &mut list);
    list.as_slice().to_vec()
}

pub fn perft(pos: &Position, depth: u32) -> u64 {
    if depth == 0 {
        return 1;
    }
    let mut list = MoveList::new();
    gen_legal(pos, &mut list);
    if depth == 1 {
        return list.len() as u64;
    }
    let mut n = 0;
    for i in 0..list.len() {
        n += perft(&pos.make_move(list.get(i)), depth - 1);
    }
    n
}
