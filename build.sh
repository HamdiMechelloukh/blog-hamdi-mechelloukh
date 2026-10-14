#!/usr/bin/env bash
# Build complet du site : HTML statique (crates/site), module WebGPU du décor (crates/gpu) dans dist/gpu/,
# puis module de l'arène (crates/arena, page /jeu) dans dist/arena/.
set -euo pipefail
cd "$(dirname "$0")"

# wasm-opt retire environ un tiers de chaque module une fois compressé (décor : 129 -> 93 Ko).
optimize() {
  wasm-opt -Oz --enable-bulk-memory --enable-nontrapping-float-to-int --enable-sign-ext --enable-mutable-globals \
    "$1" -o "$1"
}

cargo run -p site --release -- "$@"
cargo build -p gpu --release --target wasm32-unknown-unknown
wasm-bindgen --target web --no-typescript --out-dir dist/gpu target/wasm32-unknown-unknown/release/gpu.wasm
optimize dist/gpu/gpu_bg.wasm
cargo build -p arena --release --target wasm32-unknown-unknown
wasm-bindgen --target web --no-typescript --out-dir dist/arena target/wasm32-unknown-unknown/release/arena.wasm
optimize dist/arena/arena_bg.wasm
