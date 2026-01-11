#!/bin/bash

# Test e_integer function with varying precision (1-30 significant figures)
# Usage: ./test_e.sh

echo "Testing e_integer with 1-30 significant figures:"
echo "================================================"

for i in {1..30}; do
    printf "e(%2d): " "$i"
    ./target/release/lumen-lang examples/lumen/e_integer.lm "$i"
done
