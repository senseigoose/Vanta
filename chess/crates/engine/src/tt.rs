//! Transposition table: fixed-size, power-of-two, always-replace.

use crate::position::Move;
use crate::search::{MATE, MAX_PLY};

pub const NONE: u8 = 0;
pub const LOWER: u8 = 1; // score is a lower bound (fail-high)
pub const UPPER: u8 = 2; // score is an upper bound (fail-low)
pub const EXACT: u8 = 3;

#[derive(Clone, Copy, Default)]
pub struct Entry {
    pub key: u64,
    pub mv: u16, // 0 = none
    pub score: i16,
    pub depth: i8,
    pub flag: u8,
}

impl Entry {
    pub fn best_move(&self) -> Option<Move> {
        if self.mv == 0 {
            None
        } else {
            Some(Move {
                from: (self.mv & 63) as u8,
                to: ((self.mv >> 6) & 63) as u8,
                promo: ((self.mv >> 12) & 7) as u8,
            })
        }
    }
}

fn pack(m: Move) -> u16 {
    m.from as u16 | (m.to as u16) << 6 | (m.promo as u16) << 12
}

/// Mate scores are stored relative to the node, not the root.
pub fn to_tt_score(score: i32, ply: usize) -> i16 {
    let limit = MATE - MAX_PLY as i32;
    let s = if score >= limit {
        score + ply as i32
    } else if score <= -limit {
        score - ply as i32
    } else {
        score
    };
    s as i16
}

pub fn from_tt_score(score: i32, ply: usize) -> i32 {
    let limit = MATE - MAX_PLY as i32;
    if score >= limit {
        score - ply as i32
    } else if score <= -limit {
        score + ply as i32
    } else {
        score
    }
}

pub struct Tt {
    table: Vec<Entry>,
    mask: usize,
}

impl Tt {
    pub fn new(mb: usize) -> Tt {
        let bytes = mb.max(1) * 1024 * 1024;
        let size = std::mem::size_of::<Entry>();
        let mut n = (bytes / size).next_power_of_two();
        if n * size > bytes {
            n /= 2;
        }
        let n = n.max(1024);
        Tt { table: vec![Entry::default(); n], mask: n - 1 }
    }

    pub fn clear(&mut self) {
        for e in self.table.iter_mut() {
            *e = Entry::default();
        }
    }

    #[inline]
    pub fn probe(&self, key: u64) -> Option<Entry> {
        let e = self.table[key as usize & self.mask];
        if e.flag != NONE && e.key == key {
            Some(e)
        } else {
            None
        }
    }

    #[inline]
    pub fn store(&mut self, key: u64, mv: Option<Move>, score: i16, depth: i32, flag: u8) {
        let idx = key as usize & self.mask;
        let old = self.table[idx];
        let mut mv16 = mv.map(pack).unwrap_or(0);
        if mv16 == 0 && old.key == key && old.flag != NONE {
            mv16 = old.mv; // keep the previous best move if we have none now
        }
        self.table[idx] = Entry { key, mv: mv16, score, depth: depth.clamp(0, 127) as i8, flag };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_and_probe() {
        let mut tt = Tt::new(1);
        let m = Move { from: 12, to: 28, promo: 0 };
        tt.store(0xDEADBEEF, Some(m), 42, 5, EXACT);
        let e = tt.probe(0xDEADBEEF).unwrap();
        assert_eq!((e.score, e.depth, e.flag), (42, 5, EXACT));
        assert_eq!(e.best_move(), Some(m));
        assert!(tt.probe(0xDEADBEEF ^ 1).is_none());
    }

    #[test]
    fn mate_scores_round_trip() {
        let s = MATE - 7;
        let stored = to_tt_score(s, 3) as i32;
        assert_eq!(from_tt_score(stored, 3), s);
        assert_eq!(from_tt_score(stored, 5), s - 2); // same node reached 2 plies deeper
        let s = -(MATE - 4);
        assert_eq!(from_tt_score(to_tt_score(s, 2) as i32, 2), s);
    }
}
