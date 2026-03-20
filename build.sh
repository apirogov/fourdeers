#!/bin/bash

# Build script for FourDeers - supports both WASM and native builds

set -e

echo "=== FourDeers Build Script ==="
echo ""
echo "Usage: ./build.sh [wasm|native|test]"
echo "  wasm    - Build WASM version (default)"
echo "  native  - Build native executable"
echo "  test    - Run tests"
echo ""

BUILD_TYPE=${1:-wasm}

case "$BUILD_TYPE" in
    wasm)
        # Check if wasm-pack is installed
        if ! command -v wasm-pack &> /dev/null; then
            echo "Error: wasm-pack is not installed."
            echo "Install it with: cargo install wasm-pack"
            exit 1
        fi

        # Check if WASM target is added
        if ! rustup target list --installed | grep -q "wasm32-unknown-unknown"; then
            echo "WASM target not found. Adding it now..."
            rustup target add wasm32-unknown-unknown
        fi

        # Build the WASM module
        echo "Building WASM module with wasm-pack..."
        wasm-pack build \
            --target web \
            --out-dir pkg \
            --dev  # Use --release for production builds

        echo ""
        echo "✓ WASM build complete! Output in 'pkg/' directory"
        echo ""
        echo "To serve the application:"
        echo "  python3 -m http.server 8888"
        echo "  # or: miniserve --index index.html"
        echo "  # or: npx serve ."
        echo ""
        echo "Then open http://localhost:8888 in your browser"
        ;;
        
    native)
        echo "Building native executable..."
        cargo build --bin fourdeers
        
        echo ""
        echo "✓ Native build complete!"
        echo "  Binary: target/debug/fourdeers"
        echo "  Run with: ./target/debug/fourdeers"
        ;;
        
    test)
        echo "Running tests..."
        # Run just the camera tests to avoid pulling in GUI dependencies
        cd src && cargo test --lib camera::tests
        
        echo ""
        echo "✓ Tests completed"
        ;;
        
    *)
        echo "Error: Unknown build type '$BUILD_TYPE'"
        echo "Usage: ./build.sh [wasm|native|test]"
        exit 1
        ;;
esac
