#!/usr/bin/env python3
"""Add late-move pruning. Usage: python3 add_lmp.py <path to search.rs>"""
import sys
p = sys.argv[1]
s = open(p, encoding="utf-8").read()

old = "const FP_MAX_DEPTH: i32 = 4;\n"
new = old + ("/// Late-move pruning: at depth <= this, quiet moves beyond 3 + depth^2 already\n"
             "/// tried are skipped in zero-window nodes.\n"
             "const LMP_MAX_DEPTH: i32 = 5;\n")
assert s.count(old) == 1
s = s.replace(old, new)

old = """                && static_eval + self.fp_margin * depth <= alpha
                && !child.in_check()
            {
                continue;
            }
"""
new = old + """            // Late-move pruning: at shallow depth in a zero-window node, once enough quiet
            // moves have been tried (the ordering says the rest are unlikely to help),
            // skip the remaining quiet moves that do not give check. Same exclusions as
            // futility pruning; the first move is never skipped.
            if i > 0
                && ply > 0
                && is_quiet
                && !in_check
                && beta - alpha == 1
                && depth <= LMP_MAX_DEPTH
                && alpha.abs() < MATE - MAX_PLY as i32
                && nq as i32 >= 3 + depth * depth
                && !child.in_check()
            {
                continue;
            }
"""
assert s.count(old) == 1
s = s.replace(old, new)
open(p, "w", encoding="utf-8").write(s)
print("ok")
