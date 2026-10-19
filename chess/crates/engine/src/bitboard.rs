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

// Sliding attacks via hyperbola quintessence: no per-square ray loops and no
// big tables, just four precomputed line masks per square. Magic bitboards
// would be faster still; the function signatures below stay the same.

/// [0] file, [1] rank, [2] diagonal (a1-h8 direction), [3] anti-diagonal,
/// each excluding the square itself.
static LINES: [[Bb; 64]; 4] = build_lines();

const fn build_lines() -> [[Bb; 64]; 4] {
    let mut t = [[0u64; 64]; 4];
    let mut sq = 0usize;
    while sq < 64 {
        let f = (sq % 8) as i32;
        let r = (sq / 8) as i32;
        let mut o = 0usize;
        while o < 64 {
            let of = (o % 8) as i32;
            let orr = (o / 8) as i32;
            if o != sq {
                let bit = 1u64 << o;
                if of == f {
                    t[0][sq] |= bit;
                }
                if orr == r {
                    t[1][sq] |= bit;
                }
                if of - orr == f - r {
                    t[2][sq] |= bit;
                }
                if of + orr == f + r {
                    t[3][sq] |= bit;
                }
            }
            o += 1;
        }
        sq += 1;
    }
    t
}

#[inline(always)]
fn line_attacks(occ: Bb, sq: u8, mask: Bb) -> Bb {
    let s = 1u64 << sq;
    let o = occ & mask;
    let fwd = o.wrapping_sub(s);
    let rev = o.reverse_bits().wrapping_sub(s.reverse_bits());
    (fwd ^ rev.reverse_bits()) & mask
}

#[inline]
pub fn bishop_attacks(sq: u8, occ: Bb) -> Bb {
    line_attacks(occ, sq, LINES[2][sq as usize]) | line_attacks(occ, sq, LINES[3][sq as usize])
}

#[inline]
pub fn rook_attacks(sq: u8, occ: Bb) -> Bb {
    line_attacks(occ, sq, LINES[0][sq as usize]) | line_attacks(occ, sq, LINES[1][sq as usize])
}
