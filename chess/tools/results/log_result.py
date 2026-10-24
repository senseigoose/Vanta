#!/usr/bin/env python3
"""Log a result once: appends a row to docs/results.md AND updates web/data/results.json,
which the website reads. Paste the raw output of the run on stdin.

  python3 tools/results/log_result.py sprt --what "Late-move pruning" --base v0.13 --ref abc1234 \
      [--tag v0.14 --change "SEE"] [--date 2026-10-24] [--note "text"] [--dry-run] <<'EOF'
  ...paste the fastchess "Results of new vs base" block...
  EOF

  python3 tools/results/log_result.py rating --version v0.12 [--date ...] <<'EOF'
  ...paste the "Vanta vs Stockfish (UCI_LimitStrength), per level" block...
  EOF
"""
import argparse, datetime, json, re, sys
from pathlib import Path

CHESS = Path(__file__).resolve().parents[2]
MD = CHESS / "docs" / "results.md"
DATA = CHESS / "web" / "data" / "results.json"


def last(rx, text, flags=re.M):
    m = None
    for m in re.finditer(rx, text, flags):
        pass
    return m


def parse_sprt(text):
    e = last(r"^Elo:\s*(-?[\d.]+)\s*\+/-\s*([\d.]+)", text)
    g = last(r"^Games:\s*(\d+),\s*Wins:\s*(\d+),\s*Losses:\s*(\d+),\s*Draws:\s*(\d+)", text)
    l = last(r"^LLR:\s*(-?[\d.]+)[^\[]*\[([-\d.]+),\s*([-\d.]+)\]", text)
    tc = last(r"Results of new vs base \(([^,)]+)", text)
    if not (e and g and l):
        sys.exit("Could not find the Elo / Games / LLR lines. Paste the whole 'Results of new vs base' block.")
    outcome = "Inconclusive"
    done = last(r"completed - (H[01]) was accepted", text)
    if done:
        outcome = f"{done.group(1)} accepted"
    return dict(
        elo=float(e.group(1)), err=float(e.group(2)),
        games=int(g.group(1)), w=int(g.group(2)), l=int(g.group(3)), d=int(g.group(4)),
        llr=float(l.group(1)), bounds=f"[{l.group(2)}, {l.group(3)}]",
        tc=tc.group(1) if tc else "?", outcome=outcome,
    )


def parse_rating(text):
    levels = []
    for m in re.finditer(r"^\s*(\d+)\s+(\d+) games \+(\d+) -(\d+) =(\d+) score ([\d.]+)% Vanta (\d+) \+/- (\d+)", text, re.M):
        levels.append(dict(level=int(m.group(1)), score=float(m.group(6)), rating=int(m.group(7)), err=int(m.group(8)),
                           games=int(m.group(2)), excluded="excluded" in m.group(0) or "lopsided" in text.split(m.group(0))[1].split("\n")[0]))
    c = last(r"Combined estimate[^:]*:\s*(\d+)\s*\+/-\s*(\d+)", text)
    if not levels or not c:
        sys.exit("Could not find the per-level lines and the 'Combined estimate' line.")
    return levels, int(c.group(1)), int(c.group(2))


def ensure_nl(path):
    s = path.read_text(encoding="utf-8")
    if not s.endswith("\n"):
        path.write_text(s + "\n", encoding="utf-8")


def main():
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest="kind", required=True)
    a = sub.add_parser("sprt")
    a.add_argument("--what", required=True)
    a.add_argument("--base", required=True)
    a.add_argument("--ref", required=True, help="commit hash the new side was built from")
    a.add_argument("--tag"); a.add_argument("--change"); a.add_argument("--note", default="")
    r = sub.add_parser("rating")
    r.add_argument("--version", required=True)
    for p in (a, r):
        p.add_argument("--date", default=datetime.date.today().isoformat())
        p.add_argument("--dry-run", action="store_true")
    args = ap.parse_args()
    text = sys.stdin.read()
    data = json.loads(DATA.read_text(encoding="utf-8"))

    if args.kind == "sprt":
        s = parse_sprt(text)
        data["experiments"].insert(0, dict(what=args.what, vs=args.base, elo=s["elo"], err=s["err"],
                                           games=s["games"], outcome=s["outcome"], ref=args.ref))
        if args.tag:
            if not args.change:
                sys.exit("--tag needs --change")
            data["versions"].insert(0, dict(tag=args.tag, change=args.change, elo=s["elo"], err=s["err"],
                                            w=s["w"], d=s["d"], l=s["l"]))
        note = f"; {args.note}" if args.note else ""
        row = (f"| {args.date} | {args.what} (built from {args.ref}) | {args.base} | TC {s['tc']}, {s['bounds']} | "
               f"{s['outcome']}, LLR {s['llr']:.2f}, {s['elo']:+.1f} ± {s['err']:.1f} Elo, {s['games']} games{note} |")
    else:
        levels, comb, cerr = parse_rating(text)
        old = data.get("rating", {})
        prev = dict(version=old.get("version"), rating=old.get("combined", {}).get("rating"), err=old.get("combined", {}).get("err"))
        data["rating"] = dict(version=args.version, tc="10+0.1", games_per_level=levels[0]["games"],
                              levels=[dict(level=x["level"], score=x["score"], rating=x["rating"], err=x["err"]) for x in levels],
                              combined=dict(rating=comb, err=cerr), previous=prev)
        parts = "; ".join(f"{x['score']}% at {x['level']} -> {x['rating']} ± {x['err']}" + (" (excluded)" if x["excluded"] else "") for x in levels)
        row = (f"| {args.date} | Rating anchor: {args.version} vs Stockfish UCI_LimitStrength, 10+0.1, {levels[0]['games']} games per level | "
               f"Stockfish {'/'.join(str(x['level']) for x in levels)} | gauntlet | {parts}; combined {comb} ± {cerr} (statistical only) |")

    print(row)
    if args.dry_run:
        print("(dry run: nothing written)")
        return
    ensure_nl(MD)
    with MD.open("a", encoding="utf-8") as f:
        f.write(row + "\n")
    DATA.write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"wrote {MD.relative_to(CHESS.parent)} and {DATA.relative_to(CHESS.parent)}")


main()
