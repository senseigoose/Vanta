#!/usr/bin/env bash
# Gauntlet against Stockfish at limited strength, to anchor Vanta on an absolute-ish scale.
#
#   tools/rating/run.sh "2000,2400,2800"
#
# Env: VANTA_REF (git tag/commit to rate, default HEAD), TC (default 10+0.1),
#      ROUNDS per level (default 50 = 100 games), CONC (default 2),
#      BOOK (default tools/openings/book8.epd), SF (default: stockfish on PATH or /usr/games/stockfish).
# Needs fastchess and stockfish installed. Builds the working copy as Vanta.
set -euo pipefail

LEVELS="${1:?usage: run.sh <comma-separated UCI_Elo levels>}"
TC="${TC:-10+0.1}"
ROUNDS="${ROUNDS:-50}"
CONC="${CONC:-2}"

cd "$(dirname "$0")/../.."            # .../chess
CHESS_DIR="$(pwd)"
BOOK="${BOOK:-$CHESS_DIR/tools/openings/book8.epd}"
OUT="$CHESS_DIR/tools/rating/out"
mkdir -p "$OUT"
SF="${SF:-$(command -v stockfish || echo /usr/games/stockfish)}"

[ -f "$BOOK" ] || { echo "missing opening book: $BOOK"; exit 1; }
command -v fastchess >/dev/null || { echo "fastchess not on PATH"; exit 1; }
[ -x "$SF" ] || { echo "stockfish not found at $SF"; exit 1; }

echo "== stockfish: $SF"
printf 'uci\nquit\n' | "$SF" | grep -E '^id name|UCI_Elo|UCI_LimitStrength' || true

VANTA_REF="${VANTA_REF:-HEAD}"
echo "== building Vanta ($VANTA_REF = $(git rev-parse --short "$VANTA_REF"))"
WT=/tmp/vanta_rate
git worktree remove --force "$WT" 2>/dev/null || true
git worktree add --detach "$WT" "$VANTA_REF"
REL="$(git rev-parse --show-prefix)"   # e.g. "chess/"
( cd "$WT/$REL" && CARGO_TARGET_DIR=/tmp/vanta_rate_target cargo build --release -p uci )
cp /tmp/vanta_rate_target/release/uci "$OUT/vanta"
git worktree remove --force "$WT"

IFS=',' read -ra ELOS <<< "$LEVELS"
for E in "${ELOS[@]}"; do
  echo "== Vanta vs Stockfish UCI_Elo=$E, tc=$TC, $((ROUNDS * 2)) games"
  fastchess \
    -engine cmd="$OUT/vanta" name=vanta \
    -engine cmd="$SF" name=sf option.UCI_LimitStrength=true option.UCI_Elo="$E" option.Threads=1 \
    -each tc="$TC" proto=uci option.Hash=16 \
    -openings file="$BOOK" format=epd order=random \
    -rounds "$ROUNDS" -games 2 -repeat -concurrency "$CONC" \
    -pgnout file="$OUT/games_$E.pgn" \
    -ratinginterval 20 2>&1 | tee "$OUT/level_$E.log"
done

python3 "$CHESS_DIR/tools/rating/summarize.py" "$OUT" "$LEVELS" | tee "$OUT/summary.txt"
