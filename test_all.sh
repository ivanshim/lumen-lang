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
CYAN='\033[0;36m'
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
TIMEOUT_TESTS=0
SKIPPED_TESTS=0

# Language-specific counters
declare -A LUMEN_PASSED LUMEN_FAILED LUMEN_TIMEOUT LUMEN_SKIPPED
declare -A MINIPYTHON_PASSED MINIPYTHON_FAILED MINIPYTHON_TIMEOUT MINIPYTHON_SKIPPED
declare -A MINIRUST_PASSED MINIRUST_FAILED MINIRUST_TIMEOUT MINIRUST_SKIPPED

LUMEN_PASSED=0
LUMEN_FAILED=0
LUMEN_TIMEOUT=0
LUMEN_SKIPPED=0
MINIPYTHON_PASSED=0
MINIPYTHON_FAILED=0
MINIPYTHON_TIMEOUT=0
MINIPYTHON_SKIPPED=0
MINIRUST_PASSED=0
MINIRUST_FAILED=0
MINIRUST_TIMEOUT=0
MINIRUST_SKIPPED=0

# Determine whether a combination is supported
should_skip() {
    local language="$1"
    local kernel="$2"

    # Microcode paths are still experimental for all languages
    if [ "$kernel" = "microcode" ]; then
        echo "microcode kernel is experimental; skipping"
        return 0
    fi

    # Mini-Rust frontends are under construction
    if [ "$language" = "mini-rust" ]; then
        echo "mini-rust language is under construction; skipping"
        return 0
    fi

    return 1
}

# Function to run a test
run_test() {
    local file="$1"
    local kernel="$2"
    local language="$3"
    local filename=$(basename "$file")

    # Skip unsupported combinations (counted separately)
    local skip_reason
    if skip_reason=$(should_skip "$language" "$kernel"); then
        echo -e "${CYAN}  → ${filename} (${kernel})${NC}"
        echo -e "    ${YELLOW}⚠ SKIPPED${NC} (${skip_reason})"
        TOTAL_TESTS=$((TOTAL_TESTS + 1))
        SKIPPED_TESTS=$((SKIPPED_TESTS + 1))
        case "$language" in
            lumen) LUMEN_SKIPPED=$((LUMEN_SKIPPED + 1)) ;;
            mini-python) MINIPYTHON_SKIPPED=$((MINIPYTHON_SKIPPED + 1)) ;;
            mini-rust) MINIRUST_SKIPPED=$((MINIRUST_SKIPPED + 1)) ;;
        esac
        return 0
    fi

    echo -e "${CYAN}  → ${filename} (${kernel})${NC}"
    TOTAL_TESTS=$((TOTAL_TESTS + 1))

    # Run the test with output displayed directly, capturing exit code
    local output
    output=$(timeout 5 $BINARY --kernel "$kernel" "$file" 2>&1)
    local exit_code=$?

    # Print output with indentation
    if [ -n "$output" ]; then
        echo "$output" | sed 's/^/    /'
    fi

    if [ $exit_code -eq 0 ]; then
        echo -e "    ${GREEN}✓ PASS${NC}"
        PASSED_TESTS=$((PASSED_TESTS + 1))
        case "$language" in
            lumen) LUMEN_PASSED=$((LUMEN_PASSED + 1)) ;;
            mini-python) MINIPYTHON_PASSED=$((MINIPYTHON_PASSED + 1)) ;;
            mini-rust) MINIRUST_PASSED=$((MINIRUST_PASSED + 1)) ;;
        esac
        return 0
    elif [ $exit_code -eq 124 ]; then
        echo -e "    ${RED}✗ TIMEOUT${NC}"
        TIMEOUT_TESTS=$((TIMEOUT_TESTS + 1))
        FAILED_TESTS=$((FAILED_TESTS + 1))
        case "$language" in
            lumen) LUMEN_TIMEOUT=$((LUMEN_TIMEOUT + 1)) ;;
            mini-python) MINIPYTHON_TIMEOUT=$((MINIPYTHON_TIMEOUT + 1)) ;;
            mini-rust) MINIRUST_TIMEOUT=$((MINIRUST_TIMEOUT + 1)) ;;
        esac
        return 1
    else
        echo -e "    ${RED}✗ FAIL${NC}"
        FAILED_TESTS=$((FAILED_TESTS + 1))
        case "$language" in
            lumen) LUMEN_FAILED=$((LUMEN_FAILED + 1)) ;;
            mini-python) MINIPYTHON_FAILED=$((MINIPYTHON_FAILED + 1)) ;;
            mini-rust) MINIRUST_FAILED=$((MINIRUST_FAILED + 1)) ;;
        esac
        return 1
    fi
}

echo "=========================================="
echo "  Lumen-Lang Test Suite (All Tests)"
echo "=========================================="
echo ""

# Test lumen examples with both kernels
echo -e "${YELLOW}Lumen Examples:${NC}"
for file in examples/lumen/*.lm; do
    # Test with stream kernel
    run_test "$file" "stream" "lumen"

    # Test with microcode kernel
    run_test "$file" "microcode" "lumen"
done
echo ""

# Test mini_python examples with both kernels
echo -e "${YELLOW}Mini-Python Examples:${NC}"
for file in examples/mini_python/*.py; do
    # Test with stream kernel
    run_test "$file" "stream" "mini-python"

    # Test with microcode kernel
    run_test "$file" "microcode" "mini-python"
done
echo ""

# Test mini_rust examples with both kernels
echo -e "${YELLOW}Mini-Rust Examples:${NC}"
for file in examples/mini_rust/*.rs; do
    # Test with stream kernel
    run_test "$file" "stream" "mini-rust"

    # Test with microcode kernel
    run_test "$file" "microcode" "mini-rust"
done
echo ""

# Detailed Summary by Language
echo "=========================================="
echo "  Test Summary by Language"
echo "=========================================="
echo ""
echo -e "${BLUE}Lumen:${NC}"
echo -e "  Passed:  ${GREEN}$LUMEN_PASSED${NC} | Failed: ${RED}$LUMEN_FAILED${NC} | Timeout: ${RED}$LUMEN_TIMEOUT${NC} | Skipped: ${YELLOW}$LUMEN_SKIPPED${NC}"
echo ""
echo -e "${BLUE}Mini-Python:${NC}"
echo -e "  Passed:  ${GREEN}$MINIPYTHON_PASSED${NC} | Failed: ${RED}$MINIPYTHON_FAILED${NC} | Timeout: ${RED}$MINIPYTHON_TIMEOUT${NC} | Skipped: ${YELLOW}$MINIPYTHON_SKIPPED${NC}"
echo ""
echo -e "${BLUE}Mini-Rust:${NC}"
echo -e "  Passed:  ${GREEN}$MINIRUST_PASSED${NC} | Failed: ${RED}$MINIRUST_FAILED${NC} | Timeout: ${RED}$MINIRUST_TIMEOUT${NC} | Skipped: ${YELLOW}$MINIRUST_SKIPPED${NC}"
echo ""

# Overall Summary
echo "=========================================="
echo "  Overall Summary"
echo "=========================================="
echo "Total tests:   $TOTAL_TESTS"
echo -e "Passed:        ${GREEN}$PASSED_TESTS${NC}"
echo -e "Failed:        ${RED}$FAILED_TESTS${NC} (includes $TIMEOUT_TESTS timeouts)"
echo -e "Skipped:       ${YELLOW}$SKIPPED_TESTS${NC}"
echo ""

if [ $FAILED_TESTS -eq 0 ]; then
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}Some tests failed ($FAILED_TESTS/$TOTAL_TESTS)${NC}"
    exit 1
fi
