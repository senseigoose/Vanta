# Results log
| Date | Change | Baseline | SPRT bounds | Outcome |
|---|---|---|---|---|

## Benchmarks
`uci perft 5` and `uci bench` (nodes/time/nps). Record nps before and after speed work.
| 2026-10-19 | Faster movegen + stack MoveList + selection picker (v4) | v0.2 | TC 2+0.02, [0,5], 3000 games | LLR 2.35/2.94 (cap reached, not formally accepted); +16.7 ± 9.7 Elo, LOS 99.96% |
| 2026-10-19 | Null-move pruning (R=3+depth/6) | v0.3 | TC 2+0.02, [0,10] | H1 accepted, LLR 2.98, +57.1 ± 24.6 Elo, 626 games |
| 2026-10-19 | Killer moves + history heuristic | v0.4 | TC 2+0.02, [0,5] | H1 accepted, LLR 2.96, +71.4 ± 19.2 Elo, 908 games |
