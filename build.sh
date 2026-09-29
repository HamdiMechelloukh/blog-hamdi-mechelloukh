#!/usr/bin/env bash
# Build complet du site : HTML statique (crates/site) puis module WebGPU (crates/gpu) dans dist/gpu/.
set -euo pipefail
cd "$(dirname "$0")"

cargo run -p site --release -- "$@"
cargo build -p gpu --release --target wasm32-unknown-unknown
wasm-bindgen --target web --no-typescript --out-dir dist/gpu target/wasm32-unknown-unknown/release/gpu.wasm
