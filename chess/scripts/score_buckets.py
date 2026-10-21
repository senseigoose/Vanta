#!/usr/bin/env python3
"""Win/draw/loss rate of the side that is ahead, by size of the search score.

usage: python3 scripts/score_buckets.py <label> <file.bin> [more files...]

Reads Bullet ChessBoard records (32 bytes: score = i16 at byte 24, result = u8 at byte 26,
result 0/1/2 = loss/draw/win for the side to move). Scores are folded so the side
with the better score is always 'ahead'. Records of one game share a result, so
treat differences of a few points as noise.
"""
import struct
import sys

EDGES = [0, 100, 200, 300, 500, 800, 1200, 10**9]

def main():
    label, paths = sys.argv[1], sys.argv[2:]
    counts = [[0, 0, 0] for _ in EDGES[:-1]]  # [ahead loses, draw, ahead wins]
    total = 0
    for path in paths:
        data = open(path, "rb").read()
        assert len(data) % 32 == 0, path
        for i in range(len(data) // 32):
            score, result = struct.unpack_from("<hB", data, i * 32 + 24)
            if score < 0:
                score, result = -score, 2 - result  # view from the side that is ahead
            for b in range(len(EDGES) - 1):
                if EDGES[b] <= score < EDGES[b + 1]:
                    counts[b][result] += 1
                    break
            total += 1
    print(f"{label}: {total} records from {len(paths)} files")
    print(f"  {'score of side ahead':<20} {'share':>6} {'ahead loses':>12} {'draw':>7} {'ahead wins':>11}")
    for b, (l, d, w) in enumerate(counts):
        n = l + d + w
        if n == 0:
            continue
        hi = "inf" if EDGES[b + 1] >= 10**9 else str(EDGES[b + 1])
        print(f"  {EDGES[b]:>5} .. {hi:<10} {n / total:>6.1%} {l / n:>12.1%} {d / n:>7.1%} {w / n:>11.1%}")

if __name__ == "__main__":
    main()
