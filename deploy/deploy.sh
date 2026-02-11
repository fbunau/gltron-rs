#!/usr/bin/env bash
# Build WASM and deploy to Firebase Hosting
set -e

cd "$(dirname "$0")/.."

echo "=== Building WASM (release) ==="
cargo build --target wasm32-unknown-unknown --release
wasm-bindgen target/wasm32-unknown-unknown/release/gltron.wasm \
    --out-dir web/pkg --target web --no-typescript

echo ""
echo "=== Deploying to Firebase Hosting ==="
firebase deploy --only hosting

echo ""
echo "Deploy complete!"
