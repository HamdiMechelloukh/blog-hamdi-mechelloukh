#!/usr/bin/env bash
# Build complet du site : HTML statique (crates/site), module WebGPU du décor (crates/gpu) dans dist/gpu/,
# puis module de l'arène (crates/arena, page /jeu) dans dist/arena/.
set -euo pipefail
cd "$(dirname "$0")"

cargo run -p site --release -- "$@"
cargo build -p gpu --release --target wasm32-unknown-unknown
wasm-bindgen --target web --no-typescript --out-dir dist/gpu target/wasm32-unknown-unknown/release/gpu.wasm
cargo build -p arena --release --target wasm32-unknown-unknown
wasm-bindgen --target web --no-typescript --out-dir dist/arena target/wasm32-unknown-unknown/release/arena.wasm
# wgpu + backend WebGL pèse lourd : wasm-opt retire ~25 % du binaire (~13 % une fois compressé).
wasm-opt -Oz --enable-bulk-memory --enable-nontrapping-float-to-int --enable-sign-ext --enable-mutable-globals \
  dist/arena/arena_bg.wasm -o dist/arena/arena_bg.wasm
