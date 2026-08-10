#!/bin/bash
set -e

cd "$(dirname "$0")"

# Build the WASM library
echo "Building WASM..."
cargo build --target wasm32-unknown-unknown --release --manifest-path Cargo.toml

# Run wasm-bindgen to generate JS glue code
echo "Running wasm-bindgen..."
mkdir -p pkg

# Install wasm-bindgen if not available
if ! command -v wasm-bindgen &> /dev/null; then
    echo "Installing wasm-bindgen..."
    cargo install wasm-bindgen-cli
fi

wasm-bindgen --target web --out-dir ./pkg ./target/wasm32-unknown-unknown/release/web.wasm

# Copy the .wasm file to pkg directory (wasm-bindgen doesn't copy it)
cp ./target/wasm32-unknown-unknown/release/web.wasm ./pkg/web_lib.wasm

echo "Build complete!"
echo "Files in pkg/:"
ls -la ./pkg/
