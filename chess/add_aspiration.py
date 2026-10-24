#!/usr/bin/env python3
"""Add aspiration windows to the root search. Usage: python3 add_aspiration.py <path to search.rs>"""
import sys
p = sys.argv[1]
s = open(p, encoding="utf-8").read()

# 1. constants
old = "const FP_MAX_DEPTH: i32 = 4;\n"
new = old + ("/// Aspiration windows: from this depth the root is searched in a narrow window\n"
             "/// around the previous score, widening on a fail low / fail high.\n"
             "const ASP_MIN_DEPTH: i32 = 5;\n"
             "const ASP_DELTA: i32 = 25;\n")
assert s.count(old) == 1
s = s.replace(old, new)

# 2. root loop
old = """            self.root_pv = best_move;
            self.root_best = None;
            let score = self.negamax(pos, depth, -INF, INF, 0, true);
            if self.aborted {
                break;
            }
"""
new = """            self.root_pv = best_move;
            self.root_best = None;
            let mut score;
            if depth >= ASP_MIN_DEPTH {
                let mut delta = ASP_DELTA;
                let mut alpha = (best_score - delta).max(-INF);
                let mut beta = (best_score + delta).min(INF);
                loop {
                    self.root_best = None;
                    score = self.negamax(pos, depth, alpha, beta, 0, true);
                    if self.aborted {
                        break;
                    }
                    if score <= alpha {
                        // Fail low: pull alpha down, and beta toward it.
                        beta = (alpha + beta) / 2;
                        alpha = (score - delta).max(-INF);
                    } else if score >= beta {
                        // Fail high: the move that beat beta is searched first next time.
                        if self.root_best.is_some() {
                            self.root_pv = self.root_best;
                        }
                        beta = (score + delta).min(INF);
                    } else {
                        break;
                    }
                    delta += delta / 2;
                    if delta > 800 {
                        alpha = -INF;
                        beta = INF;
                    }
                }
            } else {
                score = self.negamax(pos, depth, -INF, INF, 0, true);
            }
            if self.aborted {
                break;
            }
"""
assert s.count(old) == 1
s = s.replace(old, new)
open(p, "w", encoding="utf-8").write(s)
print("ok")
