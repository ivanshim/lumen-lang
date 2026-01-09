#!/bin/bash

# Performance comparison script for stream vs microcode kernels
# Measures execution time for each test and compares kernels

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Build the project first
echo -e "${BLUE}Building lumen-lang...${NC}"
if ! cargo build --release --quiet 2>/dev/null; then
    echo -e "${RED}Build failed!${NC}"
    exit 1
fi
echo -e "${BLUE}Built successfully${NC}\n"

BINARY="./target/release/lumen-lang"

# Associative arrays for timing data
declare -A STREAM_TIMES
declare -A MICROCODE_TIMES

# Function to run a test and measure time
run_perf_test() {
    local file="$1"
    local kernel="$2"
    local language="$3"
    local filename=$(basename "$file")

    # Run test with time measurement (using /usr/bin/time if available)
    if command -v /usr/bin/time &> /dev/null; then
        # Use /usr/bin/time for more precise measurements
        output=$(/usr/bin/time -f "%e" timeout 5 $BINARY --kernel "$kernel" "$file" 2>&1)
        exit_code=$?

        # Extract timing (last line is the time from /usr/bin/time)
        if [ $exit_code -eq 0 ] || [ $exit_code -eq 124 ]; then
            timing=$(echo "$output" | tail -1)
        else
            timing="ERROR"
        fi
    else
        # Fallback to bash built-in time
        start=$(date +%s%N)
        output=$(timeout 5 $BINARY --kernel "$kernel" "$file" 2>&1)
        exit_code=$?
        end=$(date +%s%N)

        if [ $exit_code -eq 0 ] || [ $exit_code -eq 124 ]; then
            timing=$(echo "scale=3; ($end - $start) / 1000000000" | bc)
        else
            timing="ERROR"
        fi
    fi

    echo "$timing"
}

echo "=========================================="
echo "  Performance Comparison: Stream vs Microcode"
echo "=========================================="
echo ""

echo -e "${YELLOW}Lumen Examples:${NC}"
for file in examples/lumen/*.lm; do
    filename=$(basename "$file")

    stream_time=$(run_perf_test "$file" "stream" "lumen")
    microcode_time=$(run_perf_test "$file" "microcode" "lumen")

    if [[ "$stream_time" != "ERROR" && "$microcode_time" != "ERROR" ]]; then
        ratio=$(echo "scale=2; $microcode_time / $stream_time" | bc)
        if (( $(echo "$ratio > 1.0" | bc -l) )); then
            faster="${BLUE}Stream${NC} (faster)"
        else
            faster="${BLUE}Microcode${NC} (faster)"
        fi
        printf "  %-30s | Stream: %7.3fs | Microcode: %7.3fs | Ratio: %5.2fx %b\n" \
               "$filename" "$stream_time" "$microcode_time" "$ratio" "$faster"
    fi
done
echo ""

echo -e "${YELLOW}Python Examples:${NC}"
for file in examples/python/*.py; do
    filename=$(basename "$file")

    stream_time=$(run_perf_test "$file" "stream" "python_core")
    microcode_time=$(run_perf_test "$file" "microcode" "python_core")

    if [[ "$stream_time" != "ERROR" && "$microcode_time" != "ERROR" ]]; then
        ratio=$(echo "scale=2; $microcode_time / $stream_time" | bc)
        if (( $(echo "$ratio > 1.0" | bc -l) )); then
            faster="${BLUE}Stream${NC} (faster)"
        else
            faster="${BLUE}Microcode${NC} (faster)"
        fi
        printf "  %-30s | Stream: %7.3fs | Microcode: %7.3fs | Ratio: %5.2fx %b\n" \
               "$filename" "$stream_time" "$microcode_time" "$ratio" "$faster"
    fi
done
echo ""

echo -e "${YELLOW}Rust Examples:${NC}"
for file in examples/rust/*.rs; do
    filename=$(basename "$file")

    stream_time=$(run_perf_test "$file" "stream" "rust_core")
    microcode_time=$(run_perf_test "$file" "microcode" "rust_core")

    if [[ "$stream_time" != "ERROR" && "$microcode_time" != "ERROR" ]]; then
        ratio=$(echo "scale=2; $microcode_time / $stream_time" | bc)
        if (( $(echo "$ratio > 1.0" | bc -l) )); then
            faster="${BLUE}Stream${NC} (faster)"
        else
            faster="${BLUE}Microcode${NC} (faster)"
        fi
        printf "  %-30s | Stream: %7.3fs | Microcode: %7.3fs | Ratio: %5.2fx %b\n" \
               "$filename" "$stream_time" "$microcode_time" "$ratio" "$faster"
    fi
done
echo ""
