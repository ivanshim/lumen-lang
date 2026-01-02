#!/bin/bash

# lumen-lang test script
# Tests all examples for all language modules

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Build the project first
echo -e "${YELLOW}Building lumen-lang...${NC}"
cargo build --release 2>&1 | grep -E "^(Finished|error)" || true
echo ""

if [ ! -f target/release/lumen-lang ]; then
    echo -e "${RED}Build failed!${NC}"
    exit 1
fi

BINARY="./target/release/lumen-lang"
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0
SKIPPED_TESTS=0

# Language mapping: extension -> language name
declare -A LANG_MAP=(
    ["lm"]="lumen"
    ["rs"]="mini-rust"
    ["py"]="mini-python"
    ["mpy"]="mini-python"
)

# Function to run a test
run_test() {
    local file="$1"
    local extension="${file##*.}"
    local language="${LANG_MAP[$extension]}"

    if [ -z "$language" ]; then
        echo -e "${YELLOW}⊘${NC} Unknown extension: $extension ($file)"
        return 1
    fi

    echo -n "Testing $(basename $file) ... "
    TOTAL_TESTS=$((TOTAL_TESTS + 1))

    if timeout 5s $BINARY "$file" > /tmp/test_output.txt 2>&1; then
        echo -e "${GREEN}✓${NC}"
        PASSED_TESTS=$((PASSED_TESTS + 1))
        # Show full output
        if [ -s /tmp/test_output.txt ]; then
            echo "  Output:"
            sed 's/^/    /' /tmp/test_output.txt
        fi
        return 0
    else
        echo -e "${RED}✗${NC}"
        FAILED_TESTS=$((FAILED_TESTS + 1))
        # Show full error output
        echo "  Error:"
        sed 's/^/    /' /tmp/test_output.txt
        return 1
    fi
}

echo "=========================================="
echo "  Lumen-Lang Example Test Suite"
echo "=========================================="
echo ""

# Test all example files
for lang_dir in src_lumen src_mini_*; do
    if [ -d "$lang_dir/examples" ]; then
        lang_name=$(basename "$lang_dir")
        echo -e "${YELLOW}Testing ${lang_name}:${NC}"
        has_examples=0
        for example_file in "$lang_dir/examples"/*; do
            if [ -f "$example_file" ]; then
                has_examples=1
                run_test "$example_file"
            fi
        done
        if [ $has_examples -eq 0 ]; then
            echo "  No example files found"
        fi
        echo ""
    fi
done

# Summary
echo "=========================================="
echo "  Test Summary"
echo "=========================================="
echo "Total tests:   $TOTAL_TESTS"
echo -e "Passed:        ${GREEN}$PASSED_TESTS${NC}"
echo -e "Failed:        ${RED}$FAILED_TESTS${NC}"
echo -e "Skipped:       ${BLUE}$SKIPPED_TESTS${NC}"
echo ""

if [ $FAILED_TESTS -eq 0 ]; then
    echo -e "${GREEN}All active tests passed!${NC}"
    exit 0
else
    echo -e "${RED}Some tests failed!${NC}"
    exit 1
fi
