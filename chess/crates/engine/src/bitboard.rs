//! Bitboard helpers. Square index: a1 = 0, b1 = 1, ... h8 = 63.
//! Sliding attacks use simple ray walks (correct, not fast). Swap in magic
//! bitboards later once perft passes; the function signatures stay the same.

pub type Bb = u64;

pub const FILE_A: Bb = 0x0101_0101_0101_0101;
pub const FILE_B: Bb = FILE_A << 1;
pub const FILE_G: Bb = FILE_A << 6;
pub const FILE_H: Bb = FILE_A << 7;

pub struct BitIter(Bb);

impl Iterator for BitIter {
    type Item = u8;
    #[inline]
    fn next(&mut self) -> Option<u8> {
        if self.0 == 0 {
            None
        } else {
            let s = self.0.trailing_zeros() as u8;
            self.0 &= self.0 - 1;
            Some(s)
        }
    }
}

#[inline]
pub fn bits(b: Bb) -> BitIter {
    BitIter(b)
}

pub fn knight_attacks(sq: u8) -> Bb {
    let b = 1u64 << sq;
    ((b << 17) & !FILE_A)
        | ((b << 15) & !FILE_H)
        | ((b << 10) & !(FILE_A | FILE_B))
        | ((b << 6) & !(FILE_G | FILE_H))
        | ((b >> 17) & !FILE_H)
        | ((b >> 15) & !FILE_A)
        | ((b >> 10) & !(FILE_G | FILE_H))
        | ((b >> 6) & !(FILE_A | FILE_B))
}

pub fn king_attacks(sq: u8) -> Bb {
    let b = 1u64 << sq;
    let l = (b >> 1) & !FILE_H;
    let r = (b << 1) & !FILE_A;
    let m = b | l | r;
    (m << 8) | (m >> 8) | l | r
}

/// Squares attacked by a pawn of `color` (0 = white, 1 = black) standing on `sq`.
pub fn pawn_attacks(color: usize, sq: u8) -> Bb {
    let b = 1u64 << sq;
    if color == 0 {
        ((b << 7) & !FILE_H) | ((b << 9) & !FILE_A)
    } else {
        ((b >> 9) & !FILE_H) | ((b >> 7) & !FILE_A)
    }
}

fn slide(sq: u8, occ: Bb, dirs: &[(i8, i8)]) -> Bb {
    let mut out = 0u64;
    for &(df, dr) in dirs {
        let mut f = (sq & 7) as i8 + df;
        let mut r = (sq >> 3) as i8 + dr;
        while (0..8).contains(&f) && (0..8).contains(&r) {
            let s = (r * 8 + f) as u8;
            out |= 1u64 << s;
            if (occ >> s) & 1 == 1 {
                break;
            }
            f += df;
            r += dr;
        }
    }
    out
}

pub fn bishop_attacks(sq: u8, occ: Bb) -> Bb {
    slide(sq, occ, &[(1, 1), (1, -1), (-1, 1), (-1, -1)])
}

pub fn rook_attacks(sq: u8, occ: Bb) -> Bb {
    slide(sq, occ, &[(1, 0), (-1, 0), (0, 1), (0, -1)])
}
