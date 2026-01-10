#!/bin/bash

# lumen-lang Python test script
# Tests all Python examples with stream and microcode kernels

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

# Store test results: declare associative arrays for per-kernel stats
declare -A RESULTS  # format: "kernel:status" -> count
declare -a FAILED_LIST  # list of failed tests: "kernel | file"

# Initialize all combinations
for kernel in stream microcode; do
    RESULTS["${kernel}:passed"]=0
    RESULTS["${kernel}:failed"]=0
    RESULTS["${kernel}:timeout"]=0
    RESULTS["${kernel}:skipped"]=0
done

# Function to run a test
run_test() {
    local file="$1"
    local kernel="$2"
    local filename=$(basename "$file")

    echo -e "${CYAN}  → ${filename} (${kernel})${NC}"
    TOTAL_TESTS=$((TOTAL_TESTS + 1))

    # Capture start time in nanoseconds
    local start_time=$(date +%s%N)

    # Run the test with output displayed directly, capturing exit code
    local output
    output=$(timeout 30 $BINARY --kernel "$kernel" "$file" 2>&1)
    local exit_code=$?

    # Capture end time and calculate elapsed time
    local end_time=$(date +%s%N)
    local elapsed_ns=$((end_time - start_time))
    local elapsed_ms=$((elapsed_ns / 1000000))

    # Format time display using pure bash arithmetic (no bc required)
    local time_display
    if [ $elapsed_ms -lt 1000 ]; then
        time_display="${elapsed_ms}ms"
    else
        # Convert to seconds with 3 decimal places using bash arithmetic
        local sec=$((elapsed_ns / 1000000000))
        local remaining_ms=$(( (elapsed_ns % 1000000000) / 1000000 ))
        time_display=$(printf "%d.%03d" "$sec" "$remaining_ms")s
    fi

    # Print output with indentation
    if [ -n "$output" ]; then
        echo "$output" | sed 's/^/    /'
    fi

    if [ $exit_code -eq 0 ]; then
        echo -e "    ${GREEN}✓ PASS${NC} (${time_display})"
        PASSED_TESTS=$((PASSED_TESTS + 1))
        RESULTS["${kernel}:passed"]=$((RESULTS["${kernel}:passed"] + 1))
        return 0
    elif [ $exit_code -eq 124 ]; then
        echo -e "    ${RED}✗ TIMEOUT${NC} (${time_display})"
        TIMEOUT_TESTS=$((TIMEOUT_TESTS + 1))
        FAILED_TESTS=$((FAILED_TESTS + 1))
        RESULTS["${kernel}:timeout"]=$((RESULTS["${kernel}:timeout"] + 1))
        FAILED_LIST+=("${kernel} | ${filename}")
        return 1
    else
        echo -e "    ${RED}✗ FAIL${NC} (${time_display})"
        FAILED_TESTS=$((FAILED_TESTS + 1))
        RESULTS["${kernel}:failed"]=$((RESULTS["${kernel}:failed"] + 1))
        FAILED_LIST+=("${kernel} | ${filename}")
        return 1
    fi
}

echo "=========================================="
echo "  Lumen-Lang Test Suite (Python Tests)"
echo "=========================================="
echo ""

# Test python examples with all kernels
echo -e "${YELLOW}Python Examples:${NC}"
for file in examples/python/*.py; do
    for kernel in stream microcode; do
        run_test "$file" "$kernel"
    done
done
echo ""

# Detailed Summary by Kernel
echo "=========================================="
echo "  Test Summary (By Kernel)"
echo "=========================================="
echo ""

for kernel in stream microcode; do
    passed=${RESULTS["${kernel}:passed"]:-0}
    failed=${RESULTS["${kernel}:failed"]:-0}
    timeout=${RESULTS["${kernel}:timeout"]:-0}
    skipped=${RESULTS["${kernel}:skipped"]:-0}
    total=$((passed + failed + timeout + skipped))

    if [ $total -gt 0 ]; then
        status_color="${GREEN}"
        if [ $failed -gt 0 ] || [ $timeout -gt 0 ]; then
            status_color="${RED}"
        fi

        printf "%-12s: " "${kernel^}"
        printf "${status_color}"
        printf "Passed: %-2d | Failed: %-2d | Timeout: %-2d" "$passed" "$failed" "$timeout"
        printf "${NC}"
        [ $skipped -gt 0 ] && printf " | Skipped: %d" "$skipped"
        echo ""
    fi
done
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

# List failed tests if any
if [ $FAILED_TESTS -gt 0 ]; then
    echo "=========================================="
    echo "  Failed Tests (Kernel | File)"
    echo "=========================================="
    for failed_test in "${FAILED_LIST[@]}"; do
        echo -e "  ${RED}✗${NC} $failed_test"
    done
    echo ""
fi

if [ $FAILED_TESTS -eq 0 ]; then
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}Some tests failed ($FAILED_TESTS/$TOTAL_TESTS)${NC}"
    exit 1
fi
