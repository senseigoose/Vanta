# Chess engine

| Path | Purpose |
|---|---|
| `crates/engine` | Core library: board, movegen, eval, search. No I/O, WASM-safe. |
| `crates/uci` | Native binary: UCI loop (also `perft` CLI). What fastchess runs. |
| `crates/wasm` | wasm-bindgen wrapper around `engine` for the browser. |
| `crates/datagen` | Self-play data generator (native). |
| `trainer/` | Bullet config / training launch scripts. |
| `nets/` | Small released `.nnue` files, named by version. |
| `scripts/` | Python: data prep, analysis, uploads. |
| `tools/` | fastchess/Cute Chess configs, SPRT scripts, opening books. |
| `web/` | Next.js frontend (Vercel). |
| `supabase/` | SQL migrations and config. |
| `data/` | Local scratch, gitignored. Real datasets live off-laptop. |
| `docs/` | Notes, decisions, results log. |

Test: `cargo test --release -p engine`
Perft: `cargo run --release -p uci -- perft 5`
