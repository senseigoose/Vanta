# Results log
| Date | Change | Baseline | SPRT bounds | Outcome |
|---|---|---|---|---|

## Benchmarks
`uci perft 5` and `uci bench` (nodes/time/nps). Record nps before and after speed work.
| 2026-10-19 | Faster movegen + stack MoveList + selection picker (v4) | v0.2 | TC 2+0.02, [0,5], 3000 games | LLR 2.35/2.94 (cap reached, not formally accepted); +16.7 ± 9.7 Elo, LOS 99.96% |
