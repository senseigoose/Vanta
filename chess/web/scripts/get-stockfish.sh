#!/usr/bin/env bash
# Download the Stockfish.js lite single-threaded build (GPLv3) into public/stockfish.
# Run from chess/web:  bash scripts/get-stockfish.sh   then commit public/stockfish.
set -euo pipefail
VER="v19.0.1"
NAME="stockfish-19-lite-single"
BASE="https://github.com/nmrugg/stockfish.js/releases/download/$VER"
DIR="public/stockfish"
mkdir -p "$DIR"
curl -fL -o "$DIR/$NAME.js" "$BASE/$NAME.js"
curl -fL -o "$DIR/$NAME.wasm" "$BASE/$NAME.wasm"
cat > "$DIR/manifest.json" << EOF
{ "js": "$NAME.js", "name": "Stockfish 19 lite (WASM, single thread)" }
EOF
cat > "$DIR/NOTICE.txt" << EOF
Stockfish is free software under the GNU General Public License v3.
This is Stockfish.js by Nathan Rugg, built from Stockfish: https://github.com/nmrugg/stockfish.js
Stockfish source: https://github.com/official-stockfish/Stockfish
EOF
ls -l "$DIR"
