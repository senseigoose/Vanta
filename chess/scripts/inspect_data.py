#!/usr/bin/env python3
"""Sanity-check Bullet ChessBoard records (32 bytes) written by datagen.
usage: python3 scripts/inspect_data.py <file.bin> [more files...]
Independent of the Rust packer: decodes the layout from the bulletformat spec."""
import struct, sys
from collections import Counter

REC = struct.Struct("<Q16shBBB3s")  # occ, pcs, score, result, ksq, opp_ksq, extra
assert REC.size == 32

def check(path):
    data = open(path, "rb").read()
    if len(data) % 32:
        sys.exit(f"{path}: size {len(data)} is not a multiple of 32")
    n = len(data) // 32
    results, scores, pieces = Counter(), [], Counter()
    hi, lo = Counter(), Counter()  # results when the score is clearly good / clearly bad for the side to move
    bad = 0
    for i in range(n):
        occ, pcs, score, result, ksq, opp_ksq, extra = REC.unpack_from(data, i * 32)
        squares = [s for s in range(64) if occ >> s & 1]
        nibbles = [(pcs[k // 2] >> (4 * (k & 1))) & 15 for k in range(len(squares))]
        ok = len(squares) <= 32 and result in (0, 1, 2) and extra == b"\0\0\0"
        ok = ok and all(v == 0 for v in pcs[(len(squares) + 1) // 2:])  # unused nibbles are zero
        ours_k = [s for s, v in zip(squares, nibbles) if v == 5]
        theirs_k = [s for s, v in zip(squares, nibbles) if v == 13]
        ok = ok and len(ours_k) == 1 and len(theirs_k) == 1
        ok = ok and all((v & 7) < 6 for v in nibbles)
        if ok:
            ok = ours_k[0] == ksq and theirs_k[0] ^ 56 == opp_ksq
        if not ok:
            bad += 1
            if bad <= 3:
                print(f"  bad record {i}: occ={occ:#x} result={result} ksq={ksq} opp_ksq={opp_ksq}")
        results[result] += 1
        if score > 300:
            hi[result] += 1
        elif score < -300:
            lo[result] += 1
        scores.append(score)
        pieces[len(squares)] += 1
    scores.sort()
    q = lambda p: scores[int(p * (n - 1))]
    print(f"{path}: {n} records, {bad} bad")
    print(f"  result (side to move: 0 loss / 1 draw / 2 win): " + ", ".join(f"{k}: {results[k] / n:.1%}" for k in (0, 1, 2)))
    print(f"  score cp  min {scores[0]}  p10 {q(.1)}  median {q(.5)}  p90 {q(.9)}  max {scores[-1]}")
    for name, c in (("score > +300", hi), ("score < -300", lo)):
        t = sum(c.values())
        if t:
            print(f"  {name}: {t} records, loss {c[0] / t:.1%} / draw {c[1] / t:.1%} / win {c[2] / t:.1%}")
    print(f"  pieces on board: avg {sum(k * v for k, v in pieces.items()) / n:.1f}, min {min(pieces)}, max {max(pieces)}")
    return bad

if __name__ == "__main__":
    sys.exit(1 if sum(check(p) for p in sys.argv[1:]) else 0)