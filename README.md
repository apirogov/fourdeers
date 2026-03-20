# FourDeers - Stereoscopic 4D Visualization

## Description

TODO

## Prerequisites

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install wasm-pack
cargo install wasm-pack

# Add WASM target
rustup target add wasm32-unknown-unknown
```

## Building & Running

```bash
cd fourdeers
./build.sh
python3 -m http.server 8888
# Open http://localhost:8888
```
