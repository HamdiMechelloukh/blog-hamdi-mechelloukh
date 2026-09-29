#!/usr/bin/env bash
# Installation Vercel : l'image de build n'a pas Rust. wasm-bindgen est téléchargé précompilé,
# à la version exacte de Cargo.lock (la glue JS doit correspondre à la crate).
set -euo pipefail
cd "$(dirname "$0")"

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --target wasm32-unknown-unknown

version=$(grep -A1 '^name = "wasm-bindgen"$' Cargo.lock | sed -n 's/^version = "\(.*\)"$/\1/p')
archive="wasm-bindgen-${version}-x86_64-unknown-linux-musl"
curl -sSfL "https://github.com/wasm-bindgen/wasm-bindgen/releases/download/${version}/${archive}.tar.gz" | tar xz
# L'image Vercel fournit déjà Rust sous $CARGO_HOME (/rust), pas sous ~/.cargo.
mv "${archive}/wasm-bindgen" "${CARGO_HOME:-$HOME/.cargo}/bin/"
rm -rf "${archive}"
