//! Zobrist keys, generated at compile time with splitmix64 (no dependencies).

pub struct Keys {
    pub piece: [[[u64; 64]; 6]; 2], // [color][piece][square]
    pub castling: [u64; 16],        // indexed by the 4-bit castling rights
    pub ep_file: [u64; 8],
    pub side: u64,
}

const fn next(state: u64) -> (u64, u64) {
    let s = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = s;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    (s, z ^ (z >> 31))
}

const fn build() -> Keys {
    let mut k = Keys {
        piece: [[[0; 64]; 6]; 2],
        castling: [0; 16],
        ep_file: [0; 8],
        side: 0,
    };
    let mut st = 0x1234_5678_9ABC_DEF0u64;
    let mut c = 0;
    while c < 2 {
        let mut p = 0;
        while p < 6 {
            let mut s = 0;
            while s < 64 {
                let (ns, v) = next(st);
                st = ns;
                k.piece[c][p][s] = v;
                s += 1;
            }
            p += 1;
        }
        c += 1;
    }
    let mut i = 0;
    while i < 16 {
        let (ns, v) = next(st);
        st = ns;
        k.castling[i] = v;
        i += 1;
    }
    let mut f = 0;
    while f < 8 {
        let (ns, v) = next(st);
        st = ns;
        k.ep_file[f] = v;
        f += 1;
    }
    let (_, v) = next(st);
    k.side = v;
    k
}

pub static KEYS: Keys = build();
