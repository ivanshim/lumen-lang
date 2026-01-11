#!/bin/bash

# Test pi_machin function with varying precision (1-30 significant figures)
# Usage: ./test_pi.sh

# Build the project first
echo "Building lumen-lang..."
if ! cargo build --quiet 2>/dev/null; then
    echo "Build failed!"
    exit 1
fi
echo "Built successfully"
echo ""

echo "Testing pi_machin with 1-30 significant figures:"
echo "=================================================="

BINARY="./target/debug/lumen-lang"

for i in {1..30}; do
    printf "pi(%2d): " "$i"
    $BINARY examples/lumen/pi_machin.lm "$i"
done
