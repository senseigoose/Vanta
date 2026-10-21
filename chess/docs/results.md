# Results log
| Date | Change | Baseline | SPRT bounds | Outcome |
|---|---|---|---|---|

## Benchmarks
`uci perft 5` and `uci bench` (nodes/time/nps). Record nps before and after speed work.
| 2026-10-19 | Faster movegen + stack MoveList + selection picker (v4) | v0.2 | TC 2+0.02, [0,5], 3000 games | LLR 2.35/2.94 (cap reached, not formally accepted); +16.7 ± 9.7 Elo, LOS 99.96% |
| 2026-10-19 | Null-move pruning (R=3+depth/6) | v0.3 | TC 2+0.02, [0,10] | H1 accepted, LLR 2.98, +57.1 ± 24.6 Elo, 626 games |
| 2026-10-19 | Killer moves + history heuristic | v0.4 | TC 2+0.02, [0,5] | H1 accepted, LLR 2.96, +71.4 ± 19.2 Elo, 908 games |
| 2026-10-19 | Late-move reductions | v0.5 | TC 2+0.02, [0,5] | H1 accepted, LLR 2.95, +33.3 ± 13.2 Elo, 1998 games |
| 2026-10-20 | Mailbox board + pin-aware legality (speed only) | v0.6 | TC 2+0.02, [0,5] | H1 accepted, LLR 2.96, +171.4 ± 28.7 Elo, 464 games; identical node counts, 1.94x faster at depth 11 |
| 2026-10-20 | NNUE v1 (768->256x2->1, 8M self-play positions, wdl 0.3, 40 epochs), accumulator per ply | v0.7 | TC 2+0.02, [0,10] | H0 accepted, LLR -2.96, -66.9 ± 28.9 Elo, LOS 0.00%, 468 games; untagged; ~64% of v0.7 nps at depth 14 (2.0M vs 3.1M) |
| 2026-10-21 | NNUE v1 vs same build with UseNNUE=false, fixed 50000 nodes/move (diagnostic) | main (HCE) | nodes 50000, [0,10], 3000 games | reached round cap, LLR -2.02, -2.8 ± 11.1 Elo, LOS 31%; NNUE ≈ HCE per node, so the -67 at TC is the nps cost (~64% of HCE) |
| 2026-10-21 | NNUE v2 (48M positions) vs same build with UseNNUE=false, fixed 50000 nodes/move (diagnostic) | main (HCE) | nodes 50000, [0,10] | H1 accepted, LLR 2.98, +394.9 ± 95.1 Elo, LOS 100%, 166 games; v1 (8M) was -2.8 ± 11.1 on the same test; per node, not per second |
| 2026-10-21 | v0.8: NNUE (768->256x2->1, vanta-v2, 48M self-play positions, wdl 0.3, 40 epochs) | v0.7 | TC 2+0.02, [0,10] | H1 accepted, LLR 2.96, +238.1 ± 56.1 Elo, LOS 100%, 200 games; ~64% of v0.7 nps at depth 14 |
| 2026-10-21 | v0.9: build for x86-64-v3 (AVX2/BMI2) on Linux and Windows; identical nodes and score, depth-14 nps 2.0M -> 2.87M | v0.8 | TC 2+0.02, [0,10] | H1 accepted, LLR 2.95, +91.9 ± 27.9 Elo, LOS 100%, 352 games; requires an AVX2 CPU |
