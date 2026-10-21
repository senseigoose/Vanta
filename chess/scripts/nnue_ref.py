#!/usr/bin/env python3
"""Independent reference for Vanta's NNUE eval, straight from the quantised net file.

Usage (from the chess/ directory):
    python3 scripts/nnue_ref.py                       # print reference evals
    python3 scripts/nnue_ref.py target/release/uci    # also run `uci evalfen` and compare

Mirrors Bullet's Chess768 indexing and the integer inference used in the engine
(QA=255, QB=64, scale 400, truncating division). No dependencies.
"""
import struct
import subprocess
import sys

H, NIN, QA, QB, SCALE = 256, 768, 255, 64, 400
NET = "nets/vanta-v3.nnue"

FENS = [
    "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
    "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1",
    "rnbqkbnr/pppp1ppp/8/4p3/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
    "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
    "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
    "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
    "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
    "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 b - - 0 10",
    "rnb1kbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
    "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNB1KBNR b KQkq - 0 1",
    "4k3/8/8/8/8/8/4P3/4K3 w - - 0 1",
    "4k3/4p3/8/8/8/8/8/4K3 b - - 0 1",
]


def load(path):
    b = open(path, "rb").read()
    count = NIN * H + H + 2 * H + 1
    need = count * 2
    padded = -(-need // 64) * 64
    assert len(b) == padded, f"net is {len(b)} bytes, expected {padded}"
    v = struct.unpack("<%dh" % count, b[:need])
    fw = [v[i * H:(i + 1) * H] for i in range(NIN)]
    off = NIN * H
    fb = v[off:off + H]
    off += H
    ow = v[off:off + 2 * H]
    off += 2 * H
    return fw, fb, ow, v[off]


def parse_fen(fen):
    board, stm = fen.split()[:2]
    pieces = []
    rank, file = 7, 0
    for ch in board:
        if ch == "/":
            rank, file = rank - 1, 0
        elif ch.isdigit():
            file += int(ch)
        else:
            color = 0 if ch.isupper() else 1
            pc = "pnbrqk".index(ch.lower())
            pieces.append((color, pc, rank * 8 + file))
            file += 1
    return pieces, (0 if stm == "w" else 1)


def feature(persp, color, pc, sq):
    base = 0 if color == persp else 384
    return base + 64 * pc + (sq if persp == 0 else sq ^ 56)


def tdiv(a, b):  # truncating division, like Rust's `/`
    return a // b if a >= 0 else -((-a) // b)


def evaluate(net, fen):
    fw, fb, ow, ob = net
    pieces, stm = parse_fen(fen)
    acc = [list(fb), list(fb)]
    for color, pc, sq in pieces:
        for persp in (0, 1):
            col = fw[feature(persp, color, pc, sq)]
            a = acc[persp]
            for i in range(H):
                a[i] += col[i]
    us, them = acc[stm], acc[stm ^ 1]
    sc = lambda x: min(max(x, 0), QA) ** 2
    out = sum(sc(us[i]) * ow[i] + sc(them[i]) * ow[H + i] for i in range(H))
    out = tdiv(out, QA) + ob
    return tdiv(out * SCALE, QA * QB)


def engine_eval(binary, fen):
    r = subprocess.run([binary, "evalfen", fen], capture_output=True, text=True, check=True)
    for line in r.stdout.splitlines():
        if line.startswith("nnue "):
            return int(line.split()[1])
    raise RuntimeError("no nnue line in: " + r.stdout)


def main():
    net = load(NET)
    binary = sys.argv[1] if len(sys.argv) > 1 else None
    bad = 0
    for fen in FENS:
        ref = evaluate(net, fen)
        if binary:
            eng = engine_eval(binary, fen)
            flag = "ok " if eng == ref else "BAD"
            bad += eng != ref
            print(f"{flag} ref {ref:6d}  engine {eng:6d}  {fen}")
        else:
            print(f"ref {ref:6d}  {fen}")
    if binary:
        print("ALL MATCH" if bad == 0 else f"{bad} MISMATCHES")
        sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
