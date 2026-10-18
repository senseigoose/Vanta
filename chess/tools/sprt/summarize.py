import re, sys, math
pgn = open(sys.argv[1] if len(sys.argv) > 1 else "tools/sprt/out/games.pgn").read()
w = l = d = 0
for g in re.split(r"\n\n(?=\[Event)", pgn):
    white = re.search(r'\[White "([^"]+)"', g); res = re.search(r'\[Result "([^"]+)"', g)
    if not (white and res): continue
    r = res.group(1)
    if r == "1/2-1/2": d += 1
    elif r in ("1-0", "0-1"):
        new_white = white.group(1) == "new"
        if (r == "1-0") == new_white: w += 1
        else: l += 1
n = w + l + d
if n == 0: sys.exit("no finished games found")
p = (w + d / 2) / n
var = (w * (1 - p) ** 2 + l * p ** 2 + d * (0.5 - p) ** 2) / n
se = math.sqrt(var / n)
elo = lambda x: -400 * math.log10(1 / x - 1) if 0 < x < 1 else float("inf")
print(f"games {n}  W {w}  L {l}  D {d}  score {p:.3f}")
print(f"Elo (new vs base) {elo(p):+.0f}  95% range {elo(p - 1.96 * se):+.0f} to {elo(p + 1.96 * se):+.0f}")
