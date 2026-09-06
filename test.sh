#!/bin/bash

# lumen-lang test script: runs every example on the selected kernels.
# Usage: ./test.sh [--lang all|<language>] [--kernel stream|microcode] [--omit file1 file2 ...]
#        ./test.sh <file>
# Languages built into the binary are picked by file extension; languages in
# langs/extras/ are passed to the binary as `--lang <definition file>`, so
# every run of the suite exercises reading a definition at run time.
# If --lang is not specified, tests Lumen. If --kernel is not specified,
# tests both kernels. TEST_QUIET=1 prints program output only for failures.

LANGUAGES=(lumen rplumen python rust php ruby pascal c javascript swift)
declare -A DIRS=(
    [lumen]="examples/lumen" [rplumen]="examples/rplumen" [python]="examples/python" [rust]="examples/rust" [php]="examples/php"
    [ruby]="examples/ruby" [pascal]="examples/pascal"
    [c]="examples/c" [javascript]="examples/javascript" [swift]="examples/swift"
)
declare -A EXT=([lumen]=lm [rplumen]=rpl [python]=py [rust]=rs [php]=php [ruby]=rb [pascal]=pas [c]=c [javascript]=js [swift]=swift)
declare -A DISPLAY=([lumen]=Lumen [rplumen]=RPLumen [python]=Python [rust]=Rust [php]=PHP [ruby]=Ruby [pascal]=Pascal [c]=C [javascript]=JavaScript [swift]=Swift)
declare -A FLAG=(
    [php]="--lang langs/extras/php.json" [ruby]="--lang langs/extras/ruby.json"
    [pascal]="--lang langs/extras/pascal.json"
    [c]="--lang langs/extras/c.json" [javascript]="--lang langs/extras/javascript.json"
    [swift]="--lang langs/extras/swift.json"
)

RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; BLUE='\033[0;34m'; CYAN='\033[0;36m'; NC='\033[0m'

show_help() {
    echo -e "${BLUE}Lumen-Lang Test Script${NC}\n"
    echo -e "${BLUE}USAGE:${NC}"
    echo "  ./test.sh                                    Test Lumen files (default)"
    echo "  ./test.sh --help                             Show this help message"
    echo "  ./test.sh <filename>                         Test single file (searches the example directories)"
    echo "  ./test.sh --lang <language>                  Test all files of one language, or all"
    echo "  ./test.sh --kernel <kernel>                  Test with one kernel only (default: both)"
    echo "  ./test.sh --omit <file1> [file2] ...         Exclude specific files"
    echo ""
    echo -e "${BLUE}ARGUMENTS:${NC}"
    echo "  <language>              all, ${LANGUAGES[*]}"
    echo "  <kernel>                stream or microcode"
    echo ""
    echo -e "${BLUE}EXAMPLES:${NC}"
    echo "  ./test.sh --lang all                        # Test everything on both kernels"
    echo "  ./test.sh --lang pascal --kernel stream     # One language on one kernel"
    echo "  ./test.sh fibonacci_iterative.lm            # One file on both kernels"
    echo "  ./test.sh --omit factorial.lm               # Lumen except factorial"
}

LANG_FILTER=""; KERNEL_FILTER=""; SINGLE_FILE=""
declare -a OMIT_FILES=()

language_of_file() {
    local ext="${1##*.}"
    for lang in "${LANGUAGES[@]}"; do
        if [ "${EXT[$lang]}" = "$ext" ]; then echo "$lang"; return 0; fi
    done
    return 1
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --help) show_help; exit 0 ;;
        --lang)
            LANG_FILTER="$2"
            if [ "$LANG_FILTER" != "all" ] && [ -z "${DIRS[$LANG_FILTER]}" ]; then
                echo -e "${RED}Invalid language: $LANG_FILTER${NC}"; echo "Use all or one of: ${LANGUAGES[*]}"; exit 1
            fi
            shift 2 ;;
        --kernel)
            KERNEL_FILTER="$2"
            case "$KERNEL_FILTER" in
                stream|microcode) shift 2 ;;
                *) echo -e "${RED}Invalid kernel: $KERNEL_FILTER${NC}"; exit 1 ;;
            esac ;;
        --omit)
            shift
            while [[ $# -gt 0 && "$1" != --* ]]; do OMIT_FILES+=("$1"); shift; done ;;
        *)
            if [[ -f "$1" ]]; then
                SINGLE_FILE="$1"
            else
                found=$(find examples -type f -name "$1" | head -1)
                if [[ -n "$found" ]]; then
                    SINGLE_FILE="$found"
                else
                    echo -e "${RED}File not found: $1${NC}"; exit 1
                fi
            fi
            shift ;;
    esac
done

echo -e "${BLUE}Building lumen-lang...${NC}"
if ! cargo build --quiet 2>/dev/null; then echo -e "${RED}Build failed!${NC}"; exit 1; fi
echo -e "${BLUE}Built successfully${NC}\n"

BINARY="./target/debug/lumen-lang"
TOTAL_TESTS=0; PASSED_TESTS=0; FAILED_TESTS=0; TIMEOUT_TESTS=0
declare -A RESULTS
declare -a FAILED_LIST
declare -a TESTED_LANGUAGES
for lang in "${LANGUAGES[@]}"; do
    for kernel in stream microcode; do
        RESULTS["${lang}:${kernel}:passed"]=0; RESULTS["${lang}:${kernel}:failed"]=0; RESULTS["${lang}:${kernel}:timeout"]=0
    done
done

should_omit() {
    local filename; filename=$(basename "$1")
    for omit in "${OMIT_FILES[@]}"; do [[ "$filename" == "$omit" ]] && return 0; done
    return 1
}

run_test() {
    local file="$1" kernel="$2" language="$3"
    local filename; filename=$(basename "$file")
    echo -e "${CYAN}  → ${filename} (${kernel})${NC}"
    TOTAL_TESTS=$((TOTAL_TESTS + 1))

    local start_time; start_time=$(date +%s%N)
    local output
    # shellcheck disable=SC2086
    output=$(timeout 30 $BINARY --kernel "$kernel" ${FLAG[$language]} "$file" 2>&1)
    local exit_code=$?
    local elapsed_ms=$(( ($(date +%s%N) - start_time) / 1000000 ))
    local time_display
    if [ $elapsed_ms -lt 1000 ]; then time_display="${elapsed_ms}ms"; else time_display=$(printf "%d.%03d" $((elapsed_ms / 1000)) $((elapsed_ms % 1000)))s; fi

    if [ -n "$output" ] && { [ -z "$TEST_QUIET" ] || [ $exit_code -ne 0 ]; }; then
        echo "$output" | sed 's/^/    /'
    fi

    if [ $exit_code -eq 0 ]; then
        echo -e "    ${GREEN}✓ PASS${NC} (${time_display})"
        PASSED_TESTS=$((PASSED_TESTS + 1)); RESULTS["${language}:${kernel}:passed"]=$((RESULTS["${language}:${kernel}:passed"] + 1))
    elif [ $exit_code -eq 124 ]; then
        echo -e "    ${RED}✗ TIMEOUT${NC} (${time_display})"
        TIMEOUT_TESTS=$((TIMEOUT_TESTS + 1)); FAILED_TESTS=$((FAILED_TESTS + 1))
        RESULTS["${language}:${kernel}:timeout"]=$((RESULTS["${language}:${kernel}:timeout"] + 1)); FAILED_LIST+=("${language} | ${kernel} | ${filename}")
    else
        echo -e "    ${RED}✗ FAIL${NC} (${time_display})"
        FAILED_TESTS=$((FAILED_TESTS + 1))
        RESULTS["${language}:${kernel}:failed"]=$((RESULTS["${language}:${kernel}:failed"] + 1)); FAILED_LIST+=("${language} | ${kernel} | ${filename}")
    fi
}

if [ -z "$KERNEL_FILTER" ]; then test_kernels=(stream microcode); else test_kernels=("$KERNEL_FILTER"); fi

if [ -n "$SINGLE_FILE" ]; then
    title="Single File Test: $(basename "$SINGLE_FILE")"
    language=$(language_of_file "$SINGLE_FILE") || { echo -e "${RED}Unknown file type: $SINGLE_FILE${NC}"; exit 1; }
    test_languages=()
elif [ -z "$LANG_FILTER" ]; then
    title="Lumen Tests (default)"; test_languages=(lumen)
elif [ "$LANG_FILTER" = "all" ]; then
    title="All Tests"; test_languages=("${LANGUAGES[@]}")
else
    title="${DISPLAY[$LANG_FILTER]} Tests"; test_languages=("$LANG_FILTER")
fi

echo "=========================================="
echo "  Lumen-Lang Test Suite ($title)"
echo "=========================================="
echo ""

if [ -n "$SINGLE_FILE" ]; then
    echo -e "${YELLOW}Testing: $(basename "$SINGLE_FILE")${NC}"
    for kernel in "${test_kernels[@]}"; do run_test "$SINGLE_FILE" "$kernel" "$language"; done
    echo ""
    TESTED_LANGUAGES+=("$language")
else
    for lang in "${test_languages[@]}"; do
        echo -e "${YELLOW}${DISPLAY[$lang]} Examples:${NC}"
        # Every file of the language's extension under its directory, subdirectories included.
        while IFS= read -r file; do
            should_omit "$file" && continue
            for kernel in "${test_kernels[@]}"; do run_test "$file" "$kernel" "$lang"; done
        done < <(find ${DIRS[$lang]} -type f -name "*.${EXT[$lang]}" | sort)
        echo ""
        TESTED_LANGUAGES+=("$lang")
    done
fi

echo "=========================================="
echo "  Test Summary (By Language, Then Kernel)"
echo "=========================================="
echo ""
for lang in "${TESTED_LANGUAGES[@]}"; do
    echo -e "${BLUE}${DISPLAY[$lang]}:${NC}"
    for kernel in "${test_kernels[@]}"; do
        passed=${RESULTS["${lang}:${kernel}:passed"]}; failed=${RESULTS["${lang}:${kernel}:failed"]}; timeout=${RESULTS["${lang}:${kernel}:timeout"]}
        total=$((passed + failed + timeout))
        if [ $total -gt 0 ]; then
            status_color="${GREEN}"; if [ $failed -gt 0 ] || [ $timeout -gt 0 ]; then status_color="${RED}"; fi
            printf "  %-12s: " "${kernel^}"
            printf "${status_color}Passed: %-2d | Failed: %-2d | Timeout: %-2d${NC}\n" "$passed" "$failed" "$timeout"
        fi
    done
    echo ""
done

echo ""
echo "=========================================="
echo "  Overall Summary"
echo "=========================================="
echo "Total tests:   $TOTAL_TESTS"
echo -e "Passed:        ${GREEN}$PASSED_TESTS${NC}"
echo -e "Failed:        ${RED}$FAILED_TESTS${NC} (includes $TIMEOUT_TESTS timeouts)"
echo ""

if [ $FAILED_TESTS -gt 0 ]; then
    echo "=========================================="
    echo "  Failed Tests (Language | Kernel | File)"
    echo "=========================================="
    for f in "${FAILED_LIST[@]}"; do echo -e "  ${RED}✗${NC} $f"; done
    echo ""
    echo -e "${RED}Some tests failed ($FAILED_TESTS/$TOTAL_TESTS)${NC}"
    exit 1
fi
echo -e "${GREEN}All tests passed!${NC}"
exit 0
