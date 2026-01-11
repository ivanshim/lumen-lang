#!/bin/bash

# Test e_integer function with varying precision (1-30 significant figures)
# Usage: ./test_e.sh

# Build the project first
echo "Building lumen-lang..."
if ! cargo build --quiet 2>/dev/null; then
    echo "Build failed!"
    exit 1
fi
echo "Built successfully"
echo ""

echo "Testing e_integer with 1-30 significant figures:"
echo "================================================"

BINARY="./target/debug/lumen-lang"

for i in {1..30}; do
    printf "e(%2d): " "$i"
    $BINARY examples/lumen/e_integer.lm "$i"
done
