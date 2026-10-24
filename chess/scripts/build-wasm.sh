#!/usr/bin/env bash
# Build Vanta for the browser and put the result in web/public/vanta/ (commit it: Vercel only serves files).
# Run from anywhere:  bash chess/scripts/build-wasm.sh      (needs network the first time)
set -euo pipefail
cd "$(dirname "$0")/.."   # chess/
rustup target add wasm32-unknown-unknown
( cd crates/wasm && cargo build --release --target wasm32-unknown-unknown )
VER=$(grep -A1 '^name = "wasm-bindgen"$' crates/wasm/Cargo.lock | grep '^version' | head -1 | cut -d'"' -f2)
echo "wasm-bindgen library version: $VER"
HAVE=$(command -v wasm-bindgen >/dev/null && wasm-bindgen --version | awk '{print $2}' || echo none)
if [ "$HAVE" != "$VER" ]; then
  cargo install wasm-bindgen-cli --version "$VER" --locked
fi
OUT=web/public/vanta
mkdir -p "$OUT"
wasm-bindgen crates/wasm/target/wasm32-unknown-unknown/release/vanta_wasm.wasm \
  --target web --out-dir "$OUT" --out-name vanta_wasm
cat > "$OUT/manifest.json" << JSON
{ "net": "vanta-v4", "build": "$(git describe --tags --always 2>/dev/null || echo dev)" }
JSON
ls -l "$OUT"
