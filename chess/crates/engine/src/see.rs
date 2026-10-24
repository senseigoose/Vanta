//! Static exchange evaluation: would this capture win at least `threshold`
//! centipawns if both sides keep recapturing on the target square, each using
//! its cheapest attacker and stopping when continuing would lose material?
//!
//! Approximations: pins are ignored (a pinned piece may be counted as a
//! recapturer), and promotions and castling are not evaluated (`see_ge`
//! returns true for promotions; the search never asks about castling).

use crate::bitboard::*;
use crate::position::*;

/// Piece values for exchanges, indexed by PAWN..KING. The king is 0: a king
/// "capture" is only ever the last recapture, handled separately below.
pub const SEE_VALUE: [i32; 6] = [100, 300, 300, 500, 900, 0];

/// Every piece (either colour) that attacks `sq` given the occupancy `occ`.
#[inline]
fn attackers_to(pos: &Position, sq: u8, occ: Bb) -> Bb {
    (pawn_attacks(BLACK, sq) & pos.pieces[PAWN] & pos.colors[WHITE])
        | (pawn_attacks(WHITE, sq) & pos.pieces[PAWN] & pos.colors[BLACK])
        | (knight_attacks(sq) & pos.pieces[KNIGHT])
        | (king_attacks(sq) & pos.pieces[KING])
        | (bishop_attacks(sq, occ) & (pos.pieces[BISHOP] | pos.pieces[QUEEN]))
        | (rook_attacks(sq, occ) & (pos.pieces[ROOK] | pos.pieces[QUEEN]))
}

/// True if the exchange started by `m` gains at least `threshold` centipawns
/// for the side to move.
pub fn see_ge(pos: &Position, m: Move, threshold: i32) -> bool {
    if m.promo != 0 {
        return true;
    }
    let from = m.from;
    let to = m.to;
    let mover = match pos.piece_at(from) {
        Some((_, p)) => p,
        None => return true,
    };
    let is_ep = mover == PAWN && pos.ep != NO_SQ && to == pos.ep && (from & 7) != (to & 7);
    let victim = if is_ep {
        PAWN
    } else {
        match pos.piece_at(to) {
            Some((_, p)) => p,
            None => return 0 >= threshold, // quiet move: nothing is won or lost up front
        }
    };

    let mut swap = SEE_VALUE[victim] - threshold;
    if swap < 0 {
        return false;
    }
    swap = SEE_VALUE[mover] - swap;
    if swap <= 0 {
        return true;
    }

    let mut occ = pos.occupied() ^ (1u64 << from) ^ (1u64 << to);
    if is_ep {
        let cap_sq = if pos.stm == WHITE { to - 8 } else { to + 8 };
        occ ^= 1u64 << cap_sq;
    }
    let diag = pos.pieces[BISHOP] | pos.pieces[QUEEN];
    let orth = pos.pieces[ROOK] | pos.pieces[QUEEN];
    let mut attackers = attackers_to(pos, to, occ);
    let mut stm = pos.stm;
    let mut res = true;

    loop {
        stm ^= 1;
        attackers &= occ;
        let mine = attackers & pos.colors[stm];
        if mine == 0 {
            break;
        }
        res = !res;
        let r = res as i32;

        // Cheapest attacker first; removing it can uncover a slider behind it.
        let (pt, bb) = if mine & pos.pieces[PAWN] != 0 {
            (PAWN, mine & pos.pieces[PAWN])
        } else if mine & pos.pieces[KNIGHT] != 0 {
            (KNIGHT, mine & pos.pieces[KNIGHT])
        } else if mine & pos.pieces[BISHOP] != 0 {
            (BISHOP, mine & pos.pieces[BISHOP])
        } else if mine & pos.pieces[ROOK] != 0 {
            (ROOK, mine & pos.pieces[ROOK])
        } else if mine & pos.pieces[QUEEN] != 0 {
            (QUEEN, mine & pos.pieces[QUEEN])
        } else {
            // King recaptures: only legal if the other side has no attacker left.
            return if attackers & !pos.colors[stm] != 0 { !res } else { res };
        };

        swap = SEE_VALUE[pt] - swap;
        if swap < r {
            break;
        }
        occ ^= bb & bb.wrapping_neg();
        match pt {
            PAWN | BISHOP => attackers |= bishop_attacks(to, occ) & diag,
            ROOK => attackers |= rook_attacks(to, occ) & orth,
            QUEEN => attackers |= (bishop_attacks(to, occ) & diag) | (rook_attacks(to, occ) & orth),
            _ => {}
        }
    }
    res
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mv(fen: &str, uci: &str) -> (Position, Move) {
        let pos = Position::from_fen(fen).unwrap();
        let m = crate::movegen::legal_moves(&pos)
            .into_iter()
            .find(|m| m.to_uci() == uci)
            .unwrap_or_else(|| panic!("{uci} is not legal in {fen}"));
        (pos, m)
    }

    /// The exchange value is exactly `value`: it passes at `value` and fails at `value + 1`.
    fn check(fen: &str, uci: &str, value: i32) {
        let (pos, m) = mv(fen, uci);
        assert!(see_ge(&pos, m, value), "{uci} in {fen}: should be >= {value}");
        assert!(!see_ge(&pos, m, value + 1), "{uci} in {fen}: should be < {}", value + 1);
    }

    #[test]
    fn free_pawn() {
        check("4k3/8/8/3p4/8/8/8/3RK3 w - - 0 1", "d1d5", 100);
    }

    #[test]
    fn rook_takes_defended_pawn() {
        check("4k3/8/2p5/3p4/8/8/8/3RK3 w - - 0 1", "d1d5", -400);
    }

    #[test]
    fn knight_takes_pawn_defended_by_pawn() {
        check("4k3/8/4p3/3p4/8/2N5/8/4K3 w - - 0 1", "c3d5", -200);
    }

    #[test]
    fn xray_battery_counts() {
        // Rd2xd5 Rxd5 Rxd5: the rook behind joins in, so White nets a pawn.
        check("3r3k/8/8/3p4/8/8/3R4/3RK3 w - - 0 1", "d2d5", 100);
    }

    #[test]
    fn en_passant() {
        check("4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1", "e5d6", 100);
        check("4k3/2p5/8/3pP3/8/8/8/4K3 w - d6 0 1", "e5d6", 0);
    }

    #[test]
    fn black_to_move_mirrors_white() {
        check("3rk3/8/8/8/3P4/8/8/4K3 b - - 0 1", "d8d4", 100);
        check("3rk3/8/8/8/3P4/2P5/8/4K3 b - - 0 1", "d8d4", -400);
    }
}
