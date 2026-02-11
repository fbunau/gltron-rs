#!/usr/bin/env bash
# Build WASM and deploy to a Firebase preview channel (temporary URL for testing)
set -e

cd "$(dirname "$0")/.."

CHANNEL="${1:-preview}"

echo "=== Building WASM (release) ==="
cargo build --target wasm32-unknown-unknown --release
wasm-bindgen target/wasm32-unknown-unknown/release/gltron.wasm \
    --out-dir web/pkg --target web --no-typescript

echo ""
echo "=== Deploying to preview channel: $CHANNEL ==="
firebase hosting:channel:deploy "$CHANNEL" --expires 7d

echo ""
echo "Preview deployed! Link above is valid for 7 days."
