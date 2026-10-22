#!/usr/bin/env python3
"""Turn the per-level fastchess logs into a rating estimate.

usage: summarize.py <out_dir> <comma-separated levels>

For each level, rating = stockfish UCI_Elo + Vanta's measured Elo difference.
Levels where Vanta scored under 15% or over 85% are shown but excluded from the
combined estimate, because the Elo difference is unreliable that far from 50%.
The error bars are statistical only. They do not include how well the Stockfish
Elo scale transfers to this time control.
"""
import math
import re
import sys

BLOCK = re.compile(
    r"Results of (\S+) vs (\S+).*?\n"
    r"Elo: (-?[\d.]+|inf|-inf|nan) \+/- ([\d.]+|inf|nan).*?\n"
    r".*?\n"
    r"Games: (\d+), Wins: (\d+), Losses: (\d+), Draws: (\d+), Points: ([\d.]+) \(([\d.]+) %\)",
    re.S,
)

def last_block(path):
    text = open(path).read()
    blocks = list(BLOCK.finditer(text))
    return blocks[-1] if blocks else None

def main():
    out, levels = sys.argv[1], [int(x) for x in sys.argv[2].split(",")]
    rows, usable = [], []
    for lv in levels:
        try:
            b = last_block(f"{out}/level_{lv}.log")
        except FileNotFoundError:
            b = None
        if b is None:
            rows.append(f"  {lv:>6}   no result found")
            continue
        try:
            d, err = float(b.group(3)), float(b.group(4))
        except ValueError:
            rows.append(f"  {lv:>6}   unusable Elo value in log")
            continue
        games, w, l, dr = (int(b.group(i)) for i in (5, 6, 7, 8))
        score = float(b.group(10))
        est = lv + d
        reliable = 15.0 <= score <= 85.0 and math.isfinite(d) and math.isfinite(err) and err > 0
        flag = "" if reliable else "  (too lopsided, excluded)"
        rows.append(f"  {lv:>6}   {games:>5} games  +{w} -{l} ={dr}  score {score:5.1f}%   "
                    f"Vanta {est:7.0f} +/- {err:4.0f}{flag}")
        if reliable:
            usable.append((est, err))
    print("Vanta vs Stockfish (UCI_LimitStrength), per level:")
    print("  SF Elo   games                       score        implied Vanta rating")
    print("\n".join(rows))
    if usable:
        wsum = sum(1 / e**2 for _, e in usable)
        est = sum(r / e**2 for r, e in usable) / wsum
        err = math.sqrt(1 / wsum)
        print(f"\nCombined estimate (levels near 50%): {est:.0f} +/- {err:.0f}  (statistical error only)")
    else:
        print("\nNo level scored between 15% and 85%: run again with levels closer to Vanta's strength.")
    print("Caveat: Stockfish's UCI_Elo is calibrated at its own time control and playing style;")
    print("treat the result as a rough anchor, not a rating-list number.")

if __name__ == "__main__":
    main()
