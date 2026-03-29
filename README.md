# FourDeers - Stereoscopic 4D Visualization

## Description

TODO

## Prerequisites

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install just (if not already installed)
cargo install just

# Install wasm-pack and setup WASM target
just setup
```

## Building & Running

```bash
# Build WASM (dev)
just wasm

# Serve on localhost:8888
just serve
# Open http://localhost:8888
```

## Standard Commands

```bash
cargo fmt                # Format code
cargo clippy             # Lint
cargo test               # Run all tests
cargo build [--release]  # Build native binary
```

## Extra Commands

| Command | Description |
|---------|-------------|
| `just wasm` | Build WASM (dev) |
| `just wasm-release` | Build WASM (release) |
| `just serve` | Serve on localhost:8888 |
| `just setup` | Add WASM target |
