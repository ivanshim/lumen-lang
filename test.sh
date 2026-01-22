#!/bin/bash

# lumen-lang test script
# Tests examples with both stream and microcode kernels
# Usage: ./test.sh [--lang lumen|rust|python] [--kernel stream|microcode] [--omit file1.lm file2.lm ...]
#        ./test.sh <file>
# If --lang is not specified, tests all languages
# If --kernel is not specified, tests both kernels
# If --omit is provided, those files are excluded from testing
# If a file path is provided, runs just that file with selected kernel(s)

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Log output to test.log file in addition to console
# Re-execute script with tee if not already logging
if [ -z "$TEST_LOGGING" ]; then
    export TEST_LOGGING=1
    exec "$0" "$@" 2>&1 | tee test.log
    exit 1  # Safety net - should never reach here since exec replaces the process
fi

# Function to display help
show_help() {
    echo -e "${BLUE}Lumen-Lang Test Script${NC}\n"
    echo -e "${BLUE}USAGE:${NC}"
    echo "  ./test.sh                                    Test Lumen files (default)"
    echo "  ./test.sh --help                             Show this help message"
    echo "  ./test.sh <filename>                         Test single file"
    echo "  ./test.sh --lang <language>                  Test all files of specific language"
    echo "  ./test.sh --kernel <kernel>                  Test with specific kernel only"
    echo "  ./test.sh --omit <file1> [file2] ...         Test all but exclude specific files"
    echo "  ./test.sh --lang <lang> --kernel <kernel>    Combine filters"
    echo ""
    echo -e "${BLUE}ARGUMENTS:${NC}"
    echo "  <filename>              Just the filename (searches examples/ dirs)"
    echo "                          or full path (e.g., examples/lumen/fibonacci.lm)"
    echo "  <language>              all, lumen, python, or rust"
    echo "  <kernel>                stream or microcode"
    echo ""
    echo -e "${BLUE}OPTIONS:${NC}"
    echo "  --lang <language>       Test only files of specified language"
    echo "  --kernel <kernel>       Test with stream or microcode kernel only (default: both)"
    echo "  --omit <file> ...       Exclude specific files from testing"
    echo "  --help                  Display this help message"
    echo ""
    echo -e "${BLUE}EXAMPLES:${NC}"
    echo "  ./test.sh                                   # Test Lumen files (default)"
    echo "  ./test.sh fibonacci.lm                      # Test single file with both kernels"
    echo "  ./test.sh --lang all                        # Test everything"
    echo "  ./test.sh --lang python                     # Test only Python files"
    echo "  ./test.sh --kernel stream                   # Test with stream kernel only"
    echo "  ./test.sh --kernel microcode                # Test with microcode kernel only"
    echo "  ./test.sh --lang lumen --kernel stream      # Lumen files with stream kernel"
    echo "  ./test.sh --omit factorial.lm               # Test Lumen except factorial"
    echo "  ./test.sh --lang python --omit demo.py      # Combine language and omit filters"
    echo "  ./test.sh examples/lumen/pi_machin.lm      # Test with full path"
    echo ""
    echo -e "${BLUE}SEARCH DIRECTORIES:${NC}"
    echo "  - examples/lumen"
    echo "  - examples/lumen/constructs"
    echo "  - examples/lumen/libraries"
    echo "  - examples/python"
    echo "  - examples/rust"
}

# Parse command-line arguments
LANG_FILTER=""
KERNEL_FILTER=""
SINGLE_FILE=""
declare -a OMIT_FILES=()

while [[ $# -gt 0 ]]; do
    case "$1" in
        --help)
            show_help
            exit 0
            ;;
        --lang)
            LANG_FILTER="$2"
            case "$LANG_FILTER" in
                all|lumen|rust|python)
                    shift 2
                    ;;
                *)
                    echo -e "${RED}Invalid language: $LANG_FILTER${NC}"
                    echo "Usage: $0 [--lang all|lumen|rust|python] [--kernel stream|microcode] [--omit file1.lm file2.lm ...]"
                    echo "       $0 <file>"
                    exit 1
                    ;;
            esac
            ;;
        --kernel)
            KERNEL_FILTER="$2"
            case "$KERNEL_FILTER" in
                stream|microcode)
                    shift 2
                    ;;
                *)
                    echo -e "${RED}Invalid kernel: $KERNEL_FILTER${NC}"
                    echo "Usage: $0 [--lang all|lumen|rust|python] [--kernel stream|microcode] [--omit file1.lm file2.lm ...]"
                    echo "       $0 <file>"
                    exit 1
                    ;;
            esac
            ;;
        --omit)
            shift
            while [[ $# -gt 0 && "$1" != --* ]]; do
                OMIT_FILES+=("$1")
                shift
            done
            ;;
        *)
            # Check if it's a file - first try exact path, then search examples dirs
            if [[ -f "$1" ]]; then
                SINGLE_FILE="$1"
            else
                # Search for file in examples directories
                found_file=""
                for search_dir in examples/lumen examples/lumen/constructs examples/lumen/libraries examples/python examples/rust; do
                    if [[ -f "$search_dir/$1" ]]; then
                        found_file="$search_dir/$1"
                        break
                    fi
                done

                if [[ -n "$found_file" ]]; then
                    SINGLE_FILE="$found_file"
                else
                    echo -e "${RED}File not found: $1${NC}"
                    echo "Searched in: examples/lumen, examples/lumen/constructs, examples/lumen/libraries, examples/python, examples/rust"
                    echo "Usage: $0 [--lang lumen|rust|python] [--kernel stream|microcode] [--omit file1.lm file2.lm ...]"
                    echo "       $0 <filename>           (searches examples/ directories)"
                    echo "       $0 <full/path/to/file>"
                    exit 1
                fi
            fi
            shift
            ;;
    esac
done

# Build the project first
echo -e "${BLUE}Building lumen-lang...${NC}"
if ! cargo build --quiet 2>/dev/null; then
    echo -e "${RED}Build failed!${NC}"
    exit 1
fi
echo -e "${BLUE}Built successfully${NC}\n"

STREAM_BINARY="./target/debug/stream"
MICROCODE_BINARY="./target/debug/lumen-lang"
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0
TIMEOUT_TESTS=0
SKIPPED_TESTS=0

# Store test results: declare associative arrays for per-language-per-kernel stats
declare -A RESULTS  # format: "language:kernel:status" -> count
declare -a FAILED_LIST  # list of failed tests: "language | kernel | file"
declare -a TESTED_LANGUAGES  # track which languages were tested

# Initialize all combinations
for lang in lumen python_core rust_core; do
    for kernel in stream microcode; do
        RESULTS["${lang}:${kernel}:passed"]=0
        RESULTS["${lang}:${kernel}:failed"]=0
        RESULTS["${lang}:${kernel}:timeout"]=0
        RESULTS["${lang}:${kernel}:skipped"]=0
    done
done

# Function to check if a file should be omitted
should_omit() {
    local file="$1"
    local filename=$(basename "$file")
    for omit in "${OMIT_FILES[@]}"; do
        if [[ "$filename" == "$omit" ]]; then
            return 0  # true - should omit
        fi
    done
    return 1  # false - don't omit
}

# Function to run a test
run_test() {
    local file="$1"
    local kernel="$2"
    local language="$3"
    local filename=$(basename "$file")

    echo -e "${CYAN}  → ${filename} (${kernel})${NC}"
    TOTAL_TESTS=$((TOTAL_TESTS + 1))

    # Select appropriate binary
    local binary
    if [ "$kernel" = "stream" ]; then
        binary=$STREAM_BINARY
    else
        binary=$MICROCODE_BINARY
    fi

    # Capture start time in nanoseconds
    local start_time=$(date +%s%N)

    # Run the test with output displayed directly, capturing exit code
    local output
    output=$(timeout 30 $binary "$file" 2>&1)
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
        RESULTS["${language}:${kernel}:passed"]=$((RESULTS["${language}:${kernel}:passed"] + 1))
        return 0
    elif [ $exit_code -eq 124 ]; then
        echo -e "    ${RED}✗ TIMEOUT${NC} (${time_display})"
        TIMEOUT_TESTS=$((TIMEOUT_TESTS + 1))
        FAILED_TESTS=$((FAILED_TESTS + 1))
        RESULTS["${language}:${kernel}:timeout"]=$((RESULTS["${language}:${kernel}:timeout"] + 1))
        FAILED_LIST+=("${language} | ${kernel} | ${filename}")
        return 1
    else
        echo -e "    ${RED}✗ FAIL${NC} (${time_display})"
        FAILED_TESTS=$((FAILED_TESTS + 1))
        RESULTS["${language}:${kernel}:failed"]=$((RESULTS["${language}:${kernel}:failed"] + 1))
        FAILED_LIST+=("${language} | ${kernel} | ${filename}")
        return 1
    fi
}

# Determine title and test mode
if [ -n "$SINGLE_FILE" ]; then
    # Single file mode
    filename=$(basename "$SINGLE_FILE")
    title="Single File Test: $filename"

    # Detect language from file extension
    case "$SINGLE_FILE" in
        *.lm) language="lumen" ;;
        *.py) language="python_core" ;;
        *.rs) language="rust_core" ;;
        *) echo -e "${RED}Unknown file type: $SINGLE_FILE${NC}"; exit 1 ;;
    esac
else
    # Full test suite mode
    if [ -z "$LANG_FILTER" ]; then
        # Default to Lumen tests
        title="Lumen Tests (default)"
        test_languages=("lumen")
    else
        case "$LANG_FILTER" in
            all) title="All Tests"; test_languages=("lumen" "python_core" "rust_core") ;;
            lumen) title="Lumen Tests"; test_languages=("lumen") ;;
            python) title="Python Tests"; test_languages=("python_core") ;;
            rust) title="Rust Tests"; test_languages=("rust_core") ;;
        esac
    fi
fi

# Determine which kernels to test
if [ -z "$KERNEL_FILTER" ]; then
    test_kernels=("stream" "microcode")
else
    test_kernels=("$KERNEL_FILTER")
fi

echo "=========================================="
echo "  Lumen-Lang Test Suite ($title)"
echo "=========================================="
echo ""

# Run single file if specified
if [ -n "$SINGLE_FILE" ]; then
    echo -e "${YELLOW}Testing: $(basename "$SINGLE_FILE")${NC}"
    for kernel in "${test_kernels[@]}"; do
        run_test "$SINGLE_FILE" "$kernel" "$language"
    done
    echo ""
    TESTED_LANGUAGES+=("$language")
else
    # Test lumen examples if included
    if [[ " ${test_languages[@]} " =~ " lumen " ]]; then
        echo -e "${YELLOW}Lumen Examples:${NC}"
        for file in examples/lumen/*.lm examples/lumen/constructs/*.lm examples/lumen/libraries/*.lm; do
            if should_omit "$file"; then
                continue
            fi
            for kernel in "${test_kernels[@]}"; do
                run_test "$file" "$kernel" "lumen"
            done
        done
        echo ""
        TESTED_LANGUAGES+=("lumen")
    fi

    # Test python examples if included
    if [[ " ${test_languages[@]} " =~ " python_core " ]]; then
        echo -e "${YELLOW}Python Examples:${NC}"
        for file in examples/python/*.py; do
            if should_omit "$file"; then
                continue
            fi
            for kernel in "${test_kernels[@]}"; do
                run_test "$file" "$kernel" "python_core"
            done
        done
        echo ""
        TESTED_LANGUAGES+=("python_core")
    fi

    # Test rust examples if included
    if [[ " ${test_languages[@]} " =~ " rust_core " ]]; then
        echo -e "${YELLOW}Rust Examples:${NC}"
        for file in examples/rust/*.rs; do
            if should_omit "$file"; then
                continue
            fi
            for kernel in "${test_kernels[@]}"; do
                run_test "$file" "$kernel" "rust_core"
            done
        done
        echo ""
        TESTED_LANGUAGES+=("rust_core")
    fi
fi

# Detailed Summary by Language and Kernel
echo "=========================================="
echo "  Test Summary (By Language, Then Kernel)"
echo "=========================================="
echo ""

for lang in "${TESTED_LANGUAGES[@]}"; do
    case "$lang" in
        lumen) lang_display="Lumen" ;;
        python_core) lang_display="Python Core" ;;
        rust_core) lang_display="Rust Core" ;;
    esac

    echo -e "${BLUE}${lang_display}:${NC}"

    for kernel in "${test_kernels[@]}"; do
        passed=${RESULTS["${lang}:${kernel}:passed"]:-0}
        failed=${RESULTS["${lang}:${kernel}:failed"]:-0}
        timeout=${RESULTS["${lang}:${kernel}:timeout"]:-0}
        skipped=${RESULTS["${lang}:${kernel}:skipped"]:-0}
        total=$((passed + failed + timeout + skipped))

        if [ $total -gt 0 ]; then
            status_color="${GREEN}"
            if [ $failed -gt 0 ] || [ $timeout -gt 0 ]; then
                status_color="${RED}"
            fi

            printf "  %-12s: " "${kernel^}"
            printf "${status_color}"
            printf "Passed: %-2d | Failed: %-2d | Timeout: %-2d" "$passed" "$failed" "$timeout"
            printf "${NC}"
            [ $skipped -gt 0 ] && printf " | Skipped: %d" "$skipped"
            echo ""
        fi
    done
    echo ""
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
    echo "  Failed Tests (Language | Kernel | File)"
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
