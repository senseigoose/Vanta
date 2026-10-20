#!/usr/bin/env bash
# SPRT: current working copy ("new") vs a committed baseline ("base").
#
#   tools/sprt/run.sh <base-git-ref> [elo0] [elo1]
#   e.g. tools/sprt/run.sh v0.1 0 5
#
# Env: TC (default 8+0.08), CONC (default 2), BOOK (default tools/openings/book8.epd),
#      ROUNDS (default 5000). Needs `fastchess` on PATH (see tools/sprt/README.md).
set -euo pipefail

BASE_REF="${1:?usage: run.sh <base-git-ref> [elo0] [elo1]}"
ELO0="${2:-0}"
ELO1="${3:-5}"
TC="${TC:-8+0.08}"
CONC="${CONC:-2}"
ROUNDS="${ROUNDS:-5000}"

cd "$(dirname "$0")/../.."            # .../chess
CHESS_DIR="$(pwd)"
BOOK="${BOOK:-$CHESS_DIR/tools/openings/book8.epd}"
BIN="$CHESS_DIR/tools/sprt/bin"
OUT="$CHESS_DIR/tools/sprt/out"
mkdir -p "$BIN" "$OUT"

[ -f "$BOOK" ] || { echo "missing opening book: $BOOK"; echo "make one: cargo run --release -p uci -- genbook 2000 8 1 > $BOOK"; exit 1; }
command -v fastchess >/dev/null || { echo "fastchess not on PATH, see tools/sprt/README.md"; exit 1; }

echo "== building new (working copy)"
cargo build --release -p uci
cp "$CHESS_DIR/target/release/uci" "$BIN/new"

echo "== building base ($BASE_REF)"
WT=/tmp/vanta_base
git worktree remove --force "$WT" 2>/dev/null || true
git worktree add --detach "$WT" "$BASE_REF"
REL="$(git rev-parse --show-prefix)"   # e.g. "chess/"
( cd "$WT/$REL" && CARGO_TARGET_DIR=/tmp/vanta_base_target cargo build --release -p uci )
cp /tmp/vanta_base_target/release/uci "$BIN/base"
git worktree remove --force "$WT"

LIMIT="tc=$TC"
if [ -n "${NODES:-}" ]; then LIMIT="nodes=$NODES"; fi
echo "== fastchess: new vs base, $LIMIT, sprt [$ELO0, $ELO1]"
fastchess \
  -engine cmd="$BIN/new"  name=new \
  -engine cmd="$BIN/base" name=base \
  -each $LIMIT proto=uci option.Hash=16 \
  -openings file="$BOOK" format=epd order=random \
  -rounds "$ROUNDS" -games 2 -repeat -concurrency "$CONC" \
  -sprt elo0="$ELO0" elo1="$ELO1" alpha=0.05 beta=0.05 \
  -pgnout file="$OUT/games.pgn" \
  -ratinginterval 10
