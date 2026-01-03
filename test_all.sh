#!/bin/bash

# lumen-lang comprehensive test script
# Tests all examples for all languages with both stream and microcode kernels
# Output is displayed directly, not captured
# Runs ALL tests without skipping

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Build the project first
echo -e "${BLUE}Building lumen-lang...${NC}"
if ! cargo build --quiet 2>/dev/null; then
    echo -e "${RED}Build failed!${NC}"
    exit 1
fi
echo -e "${BLUE}Built successfully${NC}\n"

BINARY="./target/debug/lumen-lang"
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

# Function to run a test
run_test() {
    local file="$1"
    local kernel="$2"
    local filename=$(basename "$file")

    echo -e "${BLUE}Testing ${filename} (${kernel} kernel)${NC}"
    TOTAL_TESTS=$((TOTAL_TESTS + 1))

    # Run the test with output displayed directly
    if timeout 5 $BINARY --kernel "$kernel" "$file" 2>&1; then
        echo -e "${GREEN}✓ PASS${NC}\n"
        PASSED_TESTS=$((PASSED_TESTS + 1))
        return 0
    else
        local exit_code=$?
        if [ $exit_code -eq 124 ]; then
            echo -e "${RED}✗ TIMEOUT${NC}\n"
            FAILED_TESTS=$((FAILED_TESTS + 1))
        else
            echo -e "${RED}✗ FAIL${NC}\n"
            FAILED_TESTS=$((FAILED_TESTS + 1))
        fi
        return 1
    fi
}

echo "=========================================="
echo "  Lumen-Lang Test Suite (All Tests)"
echo "=========================================="
echo ""

# Test lumen examples with both kernels
echo -e "${YELLOW}Testing Lumen Examples:${NC}\n"
for file in examples/lumen/*.lm; do
    filename=$(basename "$file")

    # Test with stream kernel
    run_test "$file" "stream"

    # Test with microcode kernel (always attempt)
    run_test "$file" "microcode"
done

# Test mini_python examples with both kernels
echo -e "${YELLOW}Testing Mini-Python Examples:${NC}\n"
for file in examples/mini_python/*.py; do
    filename=$(basename "$file")

    # Test with stream kernel
    run_test "$file" "stream"

    # Test with microcode kernel (always attempt)
    run_test "$file" "microcode"
done

# Test mini_rust examples with both kernels
echo -e "${YELLOW}Testing Mini-Rust Examples:${NC}\n"
for file in examples/mini_rust/*.rs; do
    filename=$(basename "$file")

    # Test with stream kernel
    run_test "$file" "stream"

    # Test with microcode kernel (always attempt)
    run_test "$file" "microcode"
done

# Summary
echo "=========================================="
echo "  Test Summary"
echo "=========================================="
echo "Total tests:   $TOTAL_TESTS"
echo -e "Passed:        ${GREEN}$PASSED_TESTS${NC}"
echo -e "Failed:        ${RED}$FAILED_TESTS${NC}"
echo ""

if [ $FAILED_TESTS -eq 0 ]; then
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}Some tests failed ($FAILED_TESTS/$TOTAL_TESTS)${NC}"
    exit 1
fi
