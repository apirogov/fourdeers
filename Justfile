default:
    @just --list

wasm-pack-check:
    #!/bin/bash
    if ! command -v wasm-pack &> /dev/null; then
        echo "wasm-pack not found. Installing..."
        cargo install wasm-pack
    fi

wasm-target-check:
    #!/bin/bash
    if ! rustup target list --installed | grep -q "wasm32-unknown-unknown"; then
        echo "WASM target not found. Adding it now..."
        rustup target add wasm32-unknown-unknown
    fi

@wasm:
    just wasm-pack-check
    just wasm-target-check
    wasm-pack build --target web --out-dir pkg --dev

@wasm-release:
    just wasm-pack-check
    just wasm-target-check
    wasm-pack build --target web --out-dir pkg --release

@serve:
    npx http-server . -p 8888 -a 0.0.0.0 -c-1

@setup:
    just wasm-pack-check
    just wasm-target-check
