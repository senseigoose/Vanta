//! Handcrafted eval: material + piece-square tables (Simplified Evaluation
//! Function), with the king tapered between middlegame and endgame tables.
//! Tables are written rank 8 first (as on a board diagram), from White's view.
//! This file is the part NNUE will replace.

use crate::bitboard::bits;
use crate::position::*;

const VALUE: [i32; 5] = [100, 320, 330, 500, 900];
const PHASE: [i32; 5] = [0, 1, 1, 2, 4];

#[rustfmt::skip]
const PST: [[i32; 64]; 5] = [
    // Pawn
    [  0,  0,  0,  0,  0,  0,  0,  0,
      50, 50, 50, 50, 50, 50, 50, 50,
      10, 10, 20, 30, 30, 20, 10, 10,
       5,  5, 10, 25, 25, 10,  5,  5,
       0,  0,  0, 20, 20,  0,  0,  0,
       5, -5,-10,  0,  0,-10, -5,  5,
       5, 10, 10,-20,-20, 10, 10,  5,
       0,  0,  0,  0,  0,  0,  0,  0],
    // Knight
    [-50,-40,-30,-30,-30,-30,-40,-50,
     -40,-20,  0,  0,  0,  0,-20,-40,
     -30,  0, 10, 15, 15, 10,  0,-30,
     -30,  5, 15, 20, 20, 15,  5,-30,
     -30,  0, 15, 20, 20, 15,  0,-30,
     -30,  5, 10, 15, 15, 10,  5,-30,
     -40,-20,  0,  5,  5,  0,-20,-40,
     -50,-40,-30,-30,-30,-30,-40,-50],
    // Bishop
    [-20,-10,-10,-10,-10,-10,-10,-20,
     -10,  0,  0,  0,  0,  0,  0,-10,
     -10,  0,  5, 10, 10,  5,  0,-10,
     -10,  5,  5, 10, 10,  5,  5,-10,
     -10,  0, 10, 10, 10, 10,  0,-10,
     -10, 10, 10, 10, 10, 10, 10,-10,
     -10,  5,  0,  0,  0,  0,  5,-10,
     -20,-10,-10,-10,-10,-10,-10,-20],
    // Rook
    [  0,  0,  0,  0,  0,  0,  0,  0,
       5, 10, 10, 10, 10, 10, 10,  5,
      -5,  0,  0,  0,  0,  0,  0, -5,
      -5,  0,  0,  0,  0,  0,  0, -5,
      -5,  0,  0,  0,  0,  0,  0, -5,
      -5,  0,  0,  0,  0,  0,  0, -5,
      -5,  0,  0,  0,  0,  0,  0, -5,
       0,  0,  0,  5,  5,  0,  0,  0],
    // Queen
    [-20,-10,-10, -5, -5,-10,-10,-20,
     -10,  0,  0,  0,  0,  0,  0,-10,
     -10,  0,  5,  5,  5,  5,  0,-10,
      -5,  0,  5,  5,  5,  5,  0, -5,
       0,  0,  5,  5,  5,  5,  0, -5,
     -10,  5,  5,  5,  5,  5,  0,-10,
     -10,  0,  5,  0,  0,  0,  0,-10,
     -20,-10,-10, -5, -5,-10,-10,-20],
];

#[rustfmt::skip]
const KING_MG: [i32; 64] = [
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -20,-30,-30,-40,-40,-30,-30,-20,
    -10,-20,-20,-20,-20,-20,-20,-10,
     20, 20,  0,  0,  0,  0, 20, 20,
     20, 30, 10,  0,  0, 10, 30, 20,
];

#[rustfmt::skip]
const KING_EG: [i32; 64] = [
    -50,-40,-30,-20,-20,-30,-40,-50,
    -30,-20,-10,  0,  0,-10,-20,-30,
    -30,-10, 20, 30, 30, 20,-10,-30,
    -30,-10, 30, 40, 40, 30,-10,-30,
    -30,-10, 30, 40, 40, 30,-10,-30,
    -30,-10, 20, 30, 30, 20,-10,-30,
    -30,-30,  0,  0,  0,  0,-30,-30,
    -50,-30,-30,-30,-30,-30,-30,-50,
];

/// Score in centipawns from the side to move's point of view.
pub fn evaluate(pos: &Position) -> i32 {
    let mut score = 0i32;
    let mut phase = 0i32;
    let (mut king_mg, mut king_eg) = (0i32, 0i32);

    for color in 0..2 {
        let sign = if color == WHITE { 1 } else { -1 };
        for pc in 0..5 {
            for sq in bits(pos.pieces[pc] & pos.colors[color]) {
                // Tables are rank-8-first, so White flips the rank; Black already matches.
                let idx = if color == WHITE { (sq ^ 56) as usize } else { sq as usize };
                score += sign * (VALUE[pc] + PST[pc][idx]);
                phase += PHASE[pc];
            }
        }
        let ksq = pos.king_sq(color);
        let kidx = if color == WHITE { (ksq ^ 56) as usize } else { ksq as usize };
        king_mg += sign * KING_MG[kidx];
        king_eg += sign * KING_EG[kidx];
    }

    let phase = phase.min(24);
    score += (king_mg * phase + king_eg * (24 - phase)) / 24;

    if pos.stm == WHITE { score } else { -score }
}
