# FourDeers - Stereoscopic 4D Visualization

## Description

TODO

## Math Model (Brief)

- 4D rotations use paired unit quaternions `(q_left, q_right)` with action
  `v' = q_left * v * q_right^{-1}`.
- Camera controls intentionally split responsibilities:
  - `q_left`: in-slice 3D look orientation
  - `q_right`: 4D slice orientation (tilt + slice normal)
- Movement semantics:
  - `forward/backward/left/right/up/down` follow camera perspective in the current slice
  - `kata/ana` move along the slice normal

Detailed invariants and refactor guardrails are documented in:
- `src/rotation4d.rs`
- `src/camera.rs`

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
