use engine::bitboard::{bishop_attacks, rook_attacks};

/// Slow reference: walk each ray until a blocker.
fn reference(sq: u8, occ: u64, dirs: &[(i8, i8)]) -> u64 {
    let mut out = 0u64;
    for &(df, dr) in dirs {
        let (mut f, mut r) = ((sq & 7) as i8 + df, (sq >> 3) as i8 + dr);
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

#[test]
fn sliders_match_reference() {
    let mut x = 0x2545_F491_4F6C_DD1Du64;
    let mut next = || {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        x
    };
    for sq in 0..64u8 {
        for i in 0..400 {
            // Mix dense, sparse and empty boards.
            let occ = match i % 4 {
                0 => next(),
                1 => next() & next(),
                2 => next() & next() & next(),
                _ => 0,
            };
            assert_eq!(rook_attacks(sq, occ), reference(sq, occ, &[(1, 0), (-1, 0), (0, 1), (0, -1)]), "rook sq {sq} occ {occ:#x}");
            assert_eq!(bishop_attacks(sq, occ), reference(sq, occ, &[(1, 1), (1, -1), (-1, 1), (-1, -1)]), "bishop sq {sq} occ {occ:#x}");
        }
    }
}
