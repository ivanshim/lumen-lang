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
declare -A PYTHONCORE_PASSED PYTHONCORE_FAILED PYTHONCORE_TIMEOUT PYTHONCORE_SKIPPED
declare -A RUSTCORE_PASSED RUSTCORE_FAILED RUSTCORE_TIMEOUT RUSTCORE_SKIPPED

LUMEN_PASSED=0
LUMEN_FAILED=0
LUMEN_TIMEOUT=0
LUMEN_SKIPPED=0
PYTHONCORE_PASSED=0
PYTHONCORE_FAILED=0
PYTHONCORE_TIMEOUT=0
PYTHONCORE_SKIPPED=0
RUSTCORE_PASSED=0
RUSTCORE_FAILED=0
RUSTCORE_TIMEOUT=0
RUSTCORE_SKIPPED=0

# Determine whether a combination is supported
should_skip() {
    local language="$1"
    local kernel="$2"

    # All combinations are now supported - run all tests
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
            python_core) PYTHONCORE_SKIPPED=$((PYTHONCORE_SKIPPED + 1)) ;;
            rust_core) RUSTCORE_SKIPPED=$((RUSTCORE_SKIPPED + 1)) ;;
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
            python_core) PYTHONCORE_PASSED=$((PYTHONCORE_PASSED + 1)) ;;
            rust_core) RUSTCORE_PASSED=$((RUSTCORE_PASSED + 1)) ;;
        esac
        return 0
    elif [ $exit_code -eq 124 ]; then
        echo -e "    ${RED}✗ TIMEOUT${NC}"
        TIMEOUT_TESTS=$((TIMEOUT_TESTS + 1))
        FAILED_TESTS=$((FAILED_TESTS + 1))
        case "$language" in
            lumen) LUMEN_TIMEOUT=$((LUMEN_TIMEOUT + 1)) ;;
            python_core) PYTHONCORE_TIMEOUT=$((PYTHONCORE_TIMEOUT + 1)) ;;
            rust_core) RUSTCORE_TIMEOUT=$((RUSTCORE_TIMEOUT + 1)) ;;
        esac
        return 1
    else
        echo -e "    ${RED}✗ FAIL${NC}"
        FAILED_TESTS=$((FAILED_TESTS + 1))
        case "$language" in
            lumen) LUMEN_FAILED=$((LUMEN_FAILED + 1)) ;;
            python_core) PYTHONCORE_FAILED=$((PYTHONCORE_FAILED + 1)) ;;
            rust_core) RUSTCORE_FAILED=$((RUSTCORE_FAILED + 1)) ;;
        esac
        return 1
    fi
}

echo "=========================================="
echo "  Lumen-Lang Test Suite (All Tests)"
echo "=========================================="
echo ""

# First, run unit tests for the new opaque kernel
echo -e "${YELLOW}Opaque Kernel Unit Tests:${NC}"
if cargo test --bin opaque 2>&1 | tail -5; then
    echo -e "${GREEN}✓ Opaque kernel tests passed${NC}\n"
else
    echo -e "${RED}✗ Opaque kernel tests failed!${NC}"
    exit 1
fi

# Test lumen examples with all kernels
echo -e "${YELLOW}Lumen Examples:${NC}"
for file in examples/lumen/*.lm; do
    # Test with stream kernel
    run_test "$file" "stream" "lumen"

    # Test with microcode kernel
    run_test "$file" "microcode" "lumen"

    # Test with opaque kernel
    run_test "$file" "opaque" "lumen"
done
echo ""

# Test python examples with all kernels
echo -e "${YELLOW}Python Examples:${NC}"
for file in examples/python/*.py; do
    # Test with stream kernel
    run_test "$file" "stream" "python_core"

    # Test with microcode kernel
    run_test "$file" "microcode" "python_core"

    # Test with opaque kernel
    run_test "$file" "opaque" "python_core"
done
echo ""

# Test rust examples with all kernels
echo -e "${YELLOW}Rust Examples:${NC}"
for file in examples/rust/*.rs; do
    # Test with stream kernel
    run_test "$file" "stream" "rust_core"

    # Test with microcode kernel
    run_test "$file" "microcode" "rust_core"

    # Test with opaque kernel
    run_test "$file" "opaque" "rust_core"
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
echo -e "${BLUE}Python:${NC}"
echo -e "  Passed:  ${GREEN}$PYTHONCORE_PASSED${NC} | Failed: ${RED}$PYTHONCORE_FAILED${NC} | Timeout: ${RED}$PYTHONCORE_TIMEOUT${NC} | Skipped: ${YELLOW}$PYTHONCORE_SKIPPED${NC}"
echo ""
echo -e "${BLUE}Rust:${NC}"
echo -e "  Passed:  ${GREEN}$RUSTCORE_PASSED${NC} | Failed: ${RED}$RUSTCORE_FAILED${NC} | Timeout: ${RED}$RUSTCORE_TIMEOUT${NC} | Skipped: ${YELLOW}$RUSTCORE_SKIPPED${NC}"
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
