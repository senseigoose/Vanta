use crate::bitboard::*;
use crate::zobrist::KEYS;

pub const PAWN: usize = 0;
pub const KNIGHT: usize = 1;
pub const BISHOP: usize = 2;
pub const ROOK: usize = 3;
pub const QUEEN: usize = 4;
pub const KING: usize = 5;

pub const WHITE: usize = 0;
pub const BLACK: usize = 1;

pub const NO_SQ: u8 = 64;
const EMPTY: u8 = 0xFF;

// Castling rights bits.
pub const WK: u8 = 1;
pub const WQ: u8 = 2;
pub const BK: u8 = 4;
pub const BQ: u8 = 8;

pub const STARTPOS: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Move {
    pub from: u8,
    pub to: u8,
    /// 0 = none, otherwise KNIGHT/BISHOP/ROOK/QUEEN.
    pub promo: u8,
}

impl Move {
    pub fn to_uci(&self) -> String {
        let sq = |s: u8| format!("{}{}", (b'a' + (s & 7)) as char, (s >> 3) + 1);
        let p = match self.promo as usize {
            KNIGHT => "n",
            BISHOP => "b",
            ROOK => "r",
            QUEEN => "q",
            _ => "",
        };
        format!("{}{}{}", sq(self.from), sq(self.to), p)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Position {
    pub pieces: [Bb; 6],
    pub colors: [Bb; 2],
    pub stm: usize,
    pub castling: u8,
    pub ep: u8, // NO_SQ if none
    pub halfmove: u32,
    pub fullmove: u32,
    pub hash: u64,
    /// Mailbox: board[sq] = color * 6 + piece, or EMPTY. Kept in sync by `toggle`.
    board: [u8; 64],
}

fn castle_mask(sq: u8) -> u8 {
    match sq {
        0 => !WQ,
        4 => !(WK | WQ),
        7 => !WK,
        56 => !BQ,
        60 => !(BK | BQ),
        63 => !BK,
        _ => 0xFF,
    }
}

impl Position {
    pub fn startpos() -> Position {
        Position::from_fen(STARTPOS).unwrap()
    }

    pub fn from_fen(fen: &str) -> Result<Position, String> {
        let mut it = fen.split_whitespace();
        let board = it.next().ok_or("empty FEN")?;
        let mut p = Position {
            pieces: [0; 6],
            colors: [0; 2],
            stm: WHITE,
            castling: 0,
            ep: NO_SQ,
            halfmove: 0,
            fullmove: 1,
            hash: 0,
            board: [EMPTY; 64],
        };
        let (mut file, mut rank) = (0u8, 7i8);
        for ch in board.chars() {
            match ch {
                '/' => {
                    rank -= 1;
                    file = 0;
                }
                '1'..='8' => file += ch as u8 - b'0',
                _ => {
                    let color = if ch.is_ascii_uppercase() { WHITE } else { BLACK };
                    let piece = match ch.to_ascii_lowercase() {
                        'p' => PAWN,
                        'n' => KNIGHT,
                        'b' => BISHOP,
                        'r' => ROOK,
                        'q' => QUEEN,
                        'k' => KING,
                        _ => return Err(format!("bad piece '{ch}'")),
                    };
                    if rank < 0 || file > 7 {
                        return Err("board overflow".into());
                    }
                    p.toggle(color, piece, rank as u8 * 8 + file);
                    file += 1;
                }
            }
        }
        p.stm = match it.next().unwrap_or("w") {
            "w" => WHITE,
            "b" => BLACK,
            s => return Err(format!("bad side '{s}'")),
        };
        for ch in it.next().unwrap_or("-").chars() {
            p.castling |= match ch {
                'K' => WK,
                'Q' => WQ,
                'k' => BK,
                'q' => BQ,
                _ => 0,
            };
        }
        let ep = it.next().unwrap_or("-");
        if ep != "-" {
            let b = ep.as_bytes();
            if b.len() != 2 {
                return Err("bad ep square".into());
            }
            p.ep = (b[1] - b'1') * 8 + (b[0] - b'a');
        }
        p.halfmove = it.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        p.fullmove = it.next().and_then(|s| s.parse().ok()).unwrap_or(1);
        p.hash = p.compute_hash();
        Ok(p)
    }

    #[inline]
    pub fn occupied(&self) -> Bb {
        self.colors[0] | self.colors[1]
    }

    #[inline]
    fn toggle(&mut self, color: usize, piece: usize, sq: u8) {
        let b = 1u64 << sq;
        self.pieces[piece] ^= b;
        self.colors[color] ^= b;
        self.hash ^= KEYS.piece[color][piece][sq as usize];
        let code = (color * 6 + piece) as u8;
        let slot = &mut self.board[sq as usize];
        *slot = if *slot == code { EMPTY } else { code };
    }

    pub fn piece_on(&self, sq: u8, color: usize) -> Option<usize> {
        let c = self.board[sq as usize];
        if c != EMPTY && (c / 6) as usize == color {
            Some((c % 6) as usize)
        } else {
            None
        }
    }

    pub fn piece_at(&self, sq: u8) -> Option<(usize, usize)> {
        let c = self.board[sq as usize];
        if c == EMPTY {
            None
        } else {
            Some(((c / 6) as usize, (c % 6) as usize))
        }
    }

    pub fn king_sq(&self, color: usize) -> u8 {
        (self.pieces[KING] & self.colors[color]).trailing_zeros() as u8
    }

    /// Is `sq` attacked by any piece of color `by`?
    pub fn is_attacked(&self, sq: u8, by: usize) -> bool {
        let occ = self.occupied();
        let theirs = self.colors[by];
        if pawn_attacks(by ^ 1, sq) & self.pieces[PAWN] & theirs != 0 {
            return true;
        }
        if knight_attacks(sq) & self.pieces[KNIGHT] & theirs != 0 {
            return true;
        }
        if king_attacks(sq) & self.pieces[KING] & theirs != 0 {
            return true;
        }
        let diag = (self.pieces[BISHOP] | self.pieces[QUEEN]) & theirs;
        if bishop_attacks(sq, occ) & diag != 0 {
            return true;
        }
        let orth = (self.pieces[ROOK] | self.pieces[QUEEN]) & theirs;
        rook_attacks(sq, occ) & orth != 0
    }

    pub fn in_check(&self) -> bool {
        self.is_attacked(self.king_sq(self.stm), self.stm ^ 1)
    }

    /// Pass the move (for null-move pruning). Never call while in check.
    pub fn make_null_move(&self) -> Position {
        let mut p = *self;
        if self.ep != NO_SQ {
            p.hash ^= KEYS.ep_file[(self.ep & 7) as usize];
        }
        p.ep = NO_SQ;
        p.hash ^= KEYS.side;
        p.stm ^= 1;
        p.halfmove += 1;
        p
    }

    /// EPD-style position string (board, side, castling, ep). The ep square is
    /// only printed when a pawn can actually capture onto it.
    pub fn to_epd(&self) -> String {
        let mut s = String::new();
        for rank in (0..8u8).rev() {
            let mut empty = 0u8;
            for file in 0..8u8 {
                match self.piece_at(rank * 8 + file) {
                    Some((c, pc)) => {
                        if empty > 0 {
                            s.push_str(&empty.to_string());
                            empty = 0;
                        }
                        let ch = b"pnbrqk"[pc] as char;
                        s.push(if c == WHITE { ch.to_ascii_uppercase() } else { ch });
                    }
                    None => empty += 1,
                }
            }
            if empty > 0 {
                s.push_str(&empty.to_string());
            }
            if rank > 0 {
                s.push('/');
            }
        }
        s.push(' ');
        s.push(if self.stm == WHITE { 'w' } else { 'b' });
        s.push(' ');
        let before = s.len();
        for (bit, ch) in [(WK, 'K'), (WQ, 'Q'), (BK, 'k'), (BQ, 'q')] {
            if self.castling & bit != 0 {
                s.push(ch);
            }
        }
        if s.len() == before {
            s.push('-');
        }
        s.push(' ');
        let ep_ok = self.ep != NO_SQ
            && pawn_attacks(self.stm ^ 1, self.ep) & self.pieces[PAWN] & self.colors[self.stm] != 0;
        if ep_ok {
            s.push((b'a' + (self.ep & 7)) as char);
            s.push((b'1' + (self.ep >> 3)) as char);
        } else {
            s.push('-');
        }
        s
    }

    /// Full recompute; used by FEN parsing and tests (make_move updates incrementally).
    pub fn compute_hash(&self) -> u64 {
        let mut h = 0u64;
        for c in 0..2 {
            for pc in 0..6 {
                for sq in bits(self.pieces[pc] & self.colors[c]) {
                    h ^= KEYS.piece[c][pc][sq as usize];
                }
            }
        }
        h ^= KEYS.castling[self.castling as usize];
        if self.ep != NO_SQ {
            h ^= KEYS.ep_file[(self.ep & 7) as usize];
        }
        if self.stm == BLACK {
            h ^= KEYS.side;
        }
        h
    }

    /// Returns the new position. Assumes `m` is pseudo-legal for the side to move.
    pub fn make_move(&self, m: Move) -> Position {
        let mut p = *self;
        let us = self.stm;
        let them = us ^ 1;
        let (_, pc) = self.piece_at(m.from).expect("no piece on from-square");
        let mut reset_half = pc == PAWN;

        if let Some(cap) = self.piece_on(m.to, them) {
            p.toggle(them, cap, m.to);
            reset_half = true;
        } else if pc == PAWN && m.to == self.ep && (m.from & 7) != (m.to & 7) {
            let csq = if us == WHITE { m.to - 8 } else { m.to + 8 };
            p.toggle(them, PAWN, csq);
            reset_half = true;
        }

        p.toggle(us, pc, m.from);
        let placed = if m.promo != 0 { m.promo as usize } else { pc };
        p.toggle(us, placed, m.to);

        if pc == KING && (m.from as i8 - m.to as i8).abs() == 2 {
            let (rf, rt) = if m.to > m.from { (m.to + 1, m.to - 1) } else { (m.to - 2, m.to + 1) };
            p.toggle(us, ROOK, rf);
            p.toggle(us, ROOK, rt);
        }

        p.hash ^= KEYS.castling[self.castling as usize];
        p.castling &= castle_mask(m.from) & castle_mask(m.to);
        p.hash ^= KEYS.castling[p.castling as usize];
        if self.ep != NO_SQ {
            p.hash ^= KEYS.ep_file[(self.ep & 7) as usize];
        }
        p.ep = NO_SQ;
        if pc == PAWN && (m.from as i8 - m.to as i8).abs() == 16 {
            p.ep = (m.from + m.to) / 2;
            p.hash ^= KEYS.ep_file[(p.ep & 7) as usize];
        }
        p.hash ^= KEYS.side;
        p.halfmove = if reset_half { 0 } else { self.halfmove + 1 };
        if us == BLACK {
            p.fullmove += 1;
        }
        p.stm = them;
        p
    }
}
