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
