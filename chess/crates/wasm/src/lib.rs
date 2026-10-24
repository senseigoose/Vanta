//! Vanta in the browser. Built for wasm32-unknown-unknown by scripts/build-wasm.sh.
//!
//! A search cannot be interrupted from JavaScript (a worker is single-threaded and
//! busy while it searches), so always pass a depth, node or movetime limit.

use engine::movegen::legal_moves;
use engine::nnue;
use engine::position::Position;
use engine::search::{Info, Limits, Searcher, MATE, MAX_PLY};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Vanta {
    searcher: Searcher,
    pos: Position,
    history: Vec<u64>,
}

#[wasm_bindgen]
impl Vanta {
    /// `net` is the contents of a .nnue file (for example vanta-v4.nnue).
    #[wasm_bindgen(constructor)]
    pub fn new(net: &[u8], hash_mb: u32) -> Result<Vanta, JsError> {
        nnue::load_network(net).map_err(|e| JsError::new(&e))?;
        let mut searcher = Searcher::new(Arc::new(AtomicBool::new(false)), (hash_mb as usize).clamp(1, 256));
        searcher.set_nnue(true);
        let pos = Position::startpos();
        let history = vec![pos.hash];
        Ok(Vanta { searcher, pos, history })
    }

    pub fn new_game(&mut self) {
        self.searcher.new_game();
    }

    /// `fen` is a full FEN or the word "startpos"; `moves` are space-separated UCI moves
    /// played from it (may be empty).
    pub fn set_position(&mut self, fen: &str, moves: &str) -> Result<(), JsError> {
        let mut pos = if fen == "startpos" {
            Position::startpos()
        } else {
            Position::from_fen(fen).map_err(|e| JsError::new(&e))?
        };
        let mut history = vec![pos.hash];
        for u in moves.split_whitespace() {
            let m = legal_moves(&pos)
                .into_iter()
                .find(|m| m.to_uci() == u)
                .ok_or_else(|| JsError::new(&format!("illegal move {u}")))?;
            pos = pos.make_move(m);
            history.push(pos.hash);
        }
        self.pos = pos;
        self.history = history;
        Ok(())
    }

    /// Search the current position. A limit of 0 means "none". `on_info` is called with a JSON
    /// string after every completed depth; the return value is a JSON string:
    /// {"best":"e2e4","score":21,"mate":0,"depth":14,"nodes":123456}
    /// `score` is in centipawns for the side to move; `mate` is non-zero only for forced
    /// mates (moves to mate, negative if the side to move is being mated).
    pub fn go(&mut self, depth: u32, nodes: u32, movetime_ms: u32, on_info: &js_sys::Function) -> String {
        let limits = Limits {
            depth: if depth > 0 { Some(depth as i32) } else { None },
            movetime: if movetime_ms > 0 { Some(movetime_ms as u64) } else { None },
            nodes: if nodes > 0 { Some(nodes as u64) } else { None },
            ..Default::default()
        };
        let this = JsValue::NULL;
        let mut cb = |i: &Info| {
            let line = format!(
                "{{\"depth\":{},\"score\":{},\"mate\":{},\"nodes\":{},\"time\":{},\"best\":\"{}\"}}",
                i.depth,
                i.score,
                mate_in(i.score),
                i.nodes,
                i.time_ms,
                i.best_move.to_uci()
            );
            let _ = on_info.call1(&this, &JsValue::from_str(&line));
        };
        let res = self.searcher.search(&self.pos, self.history.clone(), &limits, &mut cb);
        let best = res.best_move.map(|m| m.to_uci()).unwrap_or_else(|| "0000".to_string());
        format!(
            "{{\"best\":\"{}\",\"score\":{},\"mate\":{},\"depth\":{},\"nodes\":{}}}",
            best,
            res.score,
            mate_in(res.score),
            res.depth,
            res.nodes
        )
    }
}

/// Moves to mate for a mate score (negative when being mated), else 0.
fn mate_in(score: i32) -> i32 {
    if score.abs() >= MATE - MAX_PLY as i32 {
        let plies = MATE - score.abs();
        let moves = (plies + 1) / 2;
        if score > 0 { moves } else { -moves }
    } else {
        0
    }
}
