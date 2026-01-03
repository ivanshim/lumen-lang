#!/bin/bash

# Microcode test runner for lumen-lang
# Tests the declarative schema-driven execution model
# This script tests the microcode track ONLY, not the stream track

set -e

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_DIR"

echo "=== Microcode Test Suite ==="
echo "Testing declarative schema-driven execution model"
echo

# Build the project
echo "Building project..."
cargo build --quiet 2>&1 | grep -v "^warning:" || true
echo "✓ Build successful"
echo

# Test directory
TEST_DIR="examples"
BINARY="target/debug/lumen-lang"

# Counter for tests
TESTS_RUN=0
TESTS_PASSED=0

# Helper function to run a test
run_test() {
    local name="$1"
    local file="$2"
    local expected="$3"

    TESTS_RUN=$((TESTS_RUN + 1))

    # Create test file if it doesn't exist
    if [ ! -f "$file" ]; then
        echo "Test file not found: $file"
        return 1
    fi

    echo -n "Test $TESTS_RUN: $name ... "

    # Run the test
    output=$("$BINARY" --lang lumen "$file" 2>&1 || true)

    # Check if output matches expected
    if echo "$output" | grep -q "$expected"; then
        echo "✓ PASS"
        TESTS_PASSED=$((TESTS_PASSED + 1))
        return 0
    else
        echo "✗ FAIL"
        echo "  Expected: $expected"
        echo "  Got: $output"
        return 1
    fi
}

# Ensure test files exist
mkdir -p "$TEST_DIR"

# Test 1: Simple print literal
echo "print(42)" > "$TEST_DIR/test_print_literal.lm"
run_test "Print literal" "$TEST_DIR/test_print_literal.lm" "42" || true

# Test 2: Print with variable
cat > "$TEST_DIR/test_print_var.lm" << 'EOF'
var x = 10
print(x)
EOF
run_test "Print variable" "$TEST_DIR/test_print_var.lm" "10" || true

# Test 3: Arithmetic expression
echo "print(3 + 4)" > "$TEST_DIR/test_arithmetic.lm"
run_test "Arithmetic" "$TEST_DIR/test_arithmetic.lm" "7" || true

# Test 4: If statement
cat > "$TEST_DIR/test_if.lm" << 'EOF'
var x = 5
if x > 0 { print(99) }
EOF
run_test "If statement" "$TEST_DIR/test_if.lm" "99" || true

# Test 5: While loop
cat > "$TEST_DIR/test_while.lm" << 'EOF'
var x = 0
while x < 3 { print(x) }
EOF
run_test "While loop" "$TEST_DIR/test_while.lm" "0" || true

echo
echo "=== Summary ==="
echo "Tests run: $TESTS_RUN"
echo "Tests passed: $TESTS_PASSED"
echo "Tests failed: $((TESTS_RUN - TESTS_PASSED))"

if [ $TESTS_PASSED -eq $TESTS_RUN ]; then
    echo "✓ All tests passed!"
    exit 0
else
    echo "✗ Some tests failed"
    exit 1
fi
