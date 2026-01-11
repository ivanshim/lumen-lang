#!/bin/bash

# Test pi_machin function with varying precision (1-30 significant figures)
# Usage: ./test_pi.sh

echo "Testing pi_machin with 1-30 significant figures:"
echo "=================================================="

for i in {1..30}; do
    printf "pi(%2d): " "$i"
    ./target/release/lumen-lang examples/lumen/pi_machin.lm "$i"
done
