#!/usr/bin/env bash
# Installation Vercel : l'image de build fournit Rust (rustup sous $CARGO_HOME=/rust), il manque la cible
# wasm32, wasm-bindgen, téléchargé précompilé à la version exacte de Cargo.lock (la glue JS doit
# correspondre à la crate), et wasm-opt (binaryen) pour alléger le module de l'arène.
set -euo pipefail
cd "$(dirname "$0")"

rustup target add wasm32-unknown-unknown

version=$(grep -A1 '^name = "wasm-bindgen"$' Cargo.lock | sed -n 's/^version = "\(.*\)"$/\1/p')
archive="wasm-bindgen-${version}-x86_64-unknown-linux-musl"
curl -sSfL "https://github.com/wasm-bindgen/wasm-bindgen/releases/download/${version}/${archive}.tar.gz" | tar xz
mv "${archive}/wasm-bindgen" "${CARGO_HOME:-$HOME/.cargo}/bin/"
rm -rf "${archive}"

binaryen_version="version_133"
binaryen="binaryen-${binaryen_version}"
curl -sSfL "https://github.com/WebAssembly/binaryen/releases/download/${binaryen_version}/${binaryen}-x86_64-linux.tar.gz" | tar xz
mv "${binaryen}/bin/wasm-opt" "${CARGO_HOME:-$HOME/.cargo}/bin/"
rm -rf "${binaryen}"
