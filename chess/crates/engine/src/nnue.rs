//! NNUE evaluation: (768 -> 256)x2 -> 1 with SCReLU, matching Bullet's `Chess768`
//! inputs and the quantised file it writes (QA = 255, QB = 64, scale 400).
//!
//! Net file layout (little-endian i16, padded to a multiple of 64 bytes):
//!   l0w: 768 x 256 (one 256-wide column per input feature)
//!   l0b: 256
//!   l1w: 512  (side-to-move half first, then the other side)
//!   l1b: 1
//!
//! Accumulators are kept per *colour* (white's view, black's view), so a null
//! move needs no update, and the side to move picks "us"/"them" at eval time.
//! The net is embedded at compile time: no file I/O, WASM-safe.

use crate::bitboard::bits;
use crate::position::*;
use std::sync::OnceLock;

pub const INPUTS: usize = 768;
pub const HIDDEN: usize = 256;
const QA: i32 = 255;
const QB: i32 = 64;
const SCALE: i32 = 400;

static NET_BYTES: &[u8] = include_bytes!("../../../nets/vanta-v4.nnue");
static NET: OnceLock<Network> = OnceLock::new();

pub struct Network {
    /// INPUTS columns of HIDDEN weights, flattened: column `f` is `fw[f*HIDDEN..(f+1)*HIDDEN]`.
    fw: Vec<i16>,
    fb: [i16; HIDDEN],
    ow: [i16; 2 * HIDDEN],
    ob: i16,
}

pub fn network() -> &'static Network {
    NET.get_or_init(|| Network::parse(NET_BYTES))
}

impl Network {
    fn parse(bytes: &[u8]) -> Network {
        let count = INPUTS * HIDDEN + HIDDEN + 2 * HIDDEN + 1;
        let need = count * 2;
        assert!(
            bytes.len() == need.next_multiple_of(64),
            "net file is {} bytes, expected {} (wrong architecture?)",
            bytes.len(),
            need.next_multiple_of(64)
        );
        let mut it = bytes.chunks_exact(2).map(|c| i16::from_le_bytes([c[0], c[1]]));
        let fw: Vec<i16> = it.by_ref().take(INPUTS * HIDDEN).collect();
        let mut fb = [0i16; HIDDEN];
        for x in fb.iter_mut() {
            *x = it.next().unwrap();
        }
        let mut ow = [0i16; 2 * HIDDEN];
        for x in ow.iter_mut() {
            *x = it.next().unwrap();
        }
        let ob = it.next().unwrap();
        Network { fw, fb, ow, ob }
    }

    /// Output in centipawns from the side to move's point of view.
    pub fn evaluate(&self, acc: &Acc, stm: usize) -> i32 {
        let us = &acc.vals[stm];
        let them = &acc.vals[stm ^ 1];
        let mut out: i32 = 0;
        for i in 0..HIDDEN {
            out += screlu(us[i]) * self.ow[i] as i32;
            out += screlu(them[i]) * self.ow[HIDDEN + i] as i32;
        }
        out /= QA; // QA*QA*QB -> QA*QB
        out += self.ob as i32;
        out *= SCALE;
        out / (QA * QB)
    }
}

#[inline]
fn screlu(x: i16) -> i32 {
    let y = (x as i32).clamp(0, QA);
    y * y
}

/// Input index of a piece as seen from `persp`'s side (Bullet's Chess768):
/// own pieces occupy 0..384, enemy pieces 384..768, 64 squares per piece type,
/// and the board is flipped vertically for Black's view.
#[inline]
fn feature(persp: usize, color: usize, pc: usize, sq: usize) -> usize {
    let base = if color == persp { 0 } else { 384 };
    let sq = if persp == WHITE { sq } else { sq ^ 56 };
    base + 64 * pc + sq
}

/// Hidden-layer sums for both colours' views.
#[derive(Clone, Copy, PartialEq)]
#[repr(C, align(64))]
pub struct Acc {
    pub vals: [[i16; HIDDEN]; 2],
}

impl Default for Acc {
    fn default() -> Acc {
        Acc { vals: [[0; HIDDEN]; 2] }
    }
}

impl Acc {
    /// Full recompute (root positions and tests).
    pub fn from_position(pos: &Position) -> Acc {
        let net = network();
        let mut a = Acc { vals: [net.fb; 2] };
        for color in 0..2 {
            for pc in 0..6 {
                for sq in bits(pos.pieces[pc] & pos.colors[color]) {
                    a.add(net, color, pc, sq as usize);
                }
            }
        }
        a
    }

    #[inline]
    fn add(&mut self, net: &Network, color: usize, pc: usize, sq: usize) {
        for persp in 0..2 {
            let f = feature(persp, color, pc, sq);
            let w = &net.fw[f * HIDDEN..(f + 1) * HIDDEN];
            for (a, &d) in self.vals[persp].iter_mut().zip(w) {
                *a = a.wrapping_add(d);
            }
        }
    }

    #[inline]
    fn sub(&mut self, net: &Network, color: usize, pc: usize, sq: usize) {
        for persp in 0..2 {
            let f = feature(persp, color, pc, sq);
            let w = &net.fw[f * HIDDEN..(f + 1) * HIDDEN];
            for (a, &d) in self.vals[persp].iter_mut().zip(w) {
                *a = a.wrapping_sub(d);
            }
        }
    }

    /// Accumulator after playing `m` in `pos` (`pos` is the position *before* the move).
    /// Mirrors `Position::make_move`: captures, en passant, promotion, castling.
    pub fn after_move(&self, pos: &Position, m: Move) -> Acc {
        let net = network();
        let mut a = *self;
        let us = pos.stm;
        let them = us ^ 1;
        let (_, pc) = pos.piece_at(m.from).expect("no piece on from-square");
        let (from, to) = (m.from as usize, m.to as usize);

        if let Some(cap) = pos.piece_on(m.to, them) {
            a.sub(net, them, cap, to);
        } else if pc == PAWN && m.to == pos.ep && (m.from & 7) != (m.to & 7) {
            let csq = if us == WHITE { to - 8 } else { to + 8 };
            a.sub(net, them, PAWN, csq);
        }

        a.sub(net, us, pc, from);
        let placed = if m.promo != 0 { m.promo as usize } else { pc };
        a.add(net, us, placed, to);

        if pc == KING && (from as i32 - to as i32).abs() == 2 {
            let (rf, rt) = if to > from { (to + 1, to - 1) } else { (to - 2, to + 1) };
            a.sub(net, us, ROOK, rf);
            a.add(net, us, ROOK, rt);
        }
        a
    }
}

/// Evaluate a position from scratch (used by `uci evalfen` and tests).
pub fn evaluate_position(pos: &Position) -> i32 {
    let acc = Acc::from_position(pos);
    network().evaluate(&acc, pos.stm)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::movegen::legal_moves;

    #[test]
    fn net_file_parses() {
        let _ = network();
    }

    /// The incremental update must equal a full recompute after every move,
    /// including captures, en passant, castling and promotions.
    #[test]
    fn incremental_matches_refresh() {
        let fens = [
            STARTPOS,
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
            "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
            "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
            "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
        ];
        let mut rng = 0x9E37_79B9_7F4A_7C15u64;
        for fen in fens {
            let mut pos = Position::from_fen(fen).unwrap();
            let mut acc = Acc::from_position(&pos);
            for _ in 0..3000 {
                let moves = legal_moves(&pos);
                if moves.is_empty() || pos.halfmove >= 100 {
                    pos = Position::from_fen(fen).unwrap();
                    acc = Acc::from_position(&pos);
                    continue;
                }
                rng ^= rng << 13;
                rng ^= rng >> 7;
                rng ^= rng << 17;
                let m = moves[(rng % moves.len() as u64) as usize];
                acc = acc.after_move(&pos, m);
                pos = pos.make_move(m);
                let fresh = Acc::from_position(&pos);
                assert!(acc.vals == fresh.vals, "accumulator mismatch from {fen} after {}", m.to_uci());
            }
        }
    }

    /// A position and its colour-mirrored twin (board flipped, colours and side
    /// swapped) must evaluate identically. This checks the perspective indexing.
    #[test]
    fn colour_mirror_symmetry() {
        let a = Position::from_fen("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1").unwrap();
        let b = Position::from_fen("rnbqkbnr/pppp1ppp/8/4p3/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
        assert_eq!(evaluate_position(&a), evaluate_position(&b));
    }

    /// Sanity on the trained net: the start position is roughly level and a
    /// missing queen is clearly bad for the side missing it.
    #[test]
    fn eval_is_sensible() {
        let start = evaluate_position(&Position::startpos());
        assert!(start.abs() < 150, "startpos eval {start}");
        let up_queen = Position::from_fen("rnb1kbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
        assert!(evaluate_position(&up_queen) > 300, "queen up eval {}", evaluate_position(&up_queen));
        let down_queen = Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNB1KBNR b KQkq - 0 1").unwrap();
        assert!(evaluate_position(&down_queen) > 300, "queen up (black to move) eval {}", evaluate_position(&down_queen));
    }
}
