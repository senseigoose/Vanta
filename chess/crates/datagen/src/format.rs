//! Bullet's `ChessBoard` binary format (bulletformat 1.x), 32 bytes per position:
//!   occ: u64 | pcs: [u8; 16] (4-bit nibbles) | score: i16 | result: u8 |
//!   ksq: u8 | opp_ksq: u8 | extra: [u8; 3]            (all little-endian)
//! Everything is from the side to move's point of view: when Black is to move the
//! board is flipped vertically and the colours swapped. Nibble = piece type
//! (P,N,B,R,Q,K = 0..5) | 8 if the piece belongs to the opponent, listed in
//! ascending square order, low nibble first. result: 0 loss, 1 draw, 2 win for
//! the side to move. opp_ksq is stored XOR 56.

use engine::position::{Position, BLACK, KING};

pub fn pack(pos: &Position, score: i16, result_stm: u8) -> [u8; 32] {
    let flip = pos.stm == BLACK;
    let f = |b: u64| if flip { b.swap_bytes() } else { b };
    let ours = f(pos.colors[pos.stm]);
    let theirs = f(pos.colors[pos.stm ^ 1]);
    let mut kinds = [0u64; 6];
    for (i, k) in kinds.iter_mut().enumerate() {
        *k = f(pos.pieces[i]);
    }

    let occ = ours | theirs;
    let mut pcs = [0u8; 16];
    let mut idx = 0usize;
    let mut rest = occ;
    while rest != 0 {
        let sq = rest.trailing_zeros();
        let bit = 1u64 << sq;
        rest &= rest - 1;
        let colour = if theirs & bit != 0 { 8u8 } else { 0 };
        let piece = (0..6).find(|&p| kinds[p] & bit != 0).expect("occupied square without piece") as u8;
        pcs[idx / 2] |= (colour | piece) << (4 * (idx & 1));
        idx += 1;
    }

    let ksq = (ours & kinds[KING]).trailing_zeros() as u8;
    let opp_ksq = (theirs & kinds[KING]).trailing_zeros() as u8 ^ 56;

    let mut out = [0u8; 32];
    out[0..8].copy_from_slice(&occ.to_le_bytes());
    out[8..24].copy_from_slice(&pcs);
    out[24..26].copy_from_slice(&score.to_le_bytes());
    out[26] = result_stm;
    out[27] = ksq;
    out[28] = opp_ksq;
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(fen: &str) -> Position {
        Position::from_fen(fen).unwrap()
    }

    #[test]
    fn startpos_layout() {
        let b = pack(&Position::startpos(), 25, 1);
        assert_eq!(u64::from_le_bytes(b[0..8].try_into().unwrap()), 0xFFFF_0000_0000_FFFF);
        // a1 rook (3) + b1 knight (1), then c1 bishop (2) + d1 queen (4).
        assert_eq!(b[8], 0x13);
        assert_eq!(b[9], 0x42);
        assert_eq!(i16::from_le_bytes(b[24..26].try_into().unwrap()), 25);
        assert_eq!((b[26], b[27], b[28]), (1, 4, 4)); // e1 king; opponent e8 ^ 56 = 4
        assert_eq!(&b[29..32], &[0, 0, 0]);
        assert_eq!(b.len(), 32);
    }

    #[test]
    fn black_to_move_is_a_mirror_of_white_to_move() {
        // Same position seen from each side: ranks reversed, colours swapped.
        let white = p("4k3/8/8/8/8/8/4P3/R3K2R w - - 0 1");
        let black = p("r3k2r/4p3/8/8/8/8/8/4K3 b - - 0 1");
        assert_eq!(pack(&white, -40, 2), pack(&black, -40, 2));
    }
}
