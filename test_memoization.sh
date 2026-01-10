#!/bin/bash

echo "======================================"
echo "Stream Kernel Memoization Test"
echo "======================================"
echo ""

FIBONACCI_RECURSIVE="/home/user/lumen-lang/examples/lumen/fibonacci_recursive.lm"

echo "Test 1: WITHOUT memoization (LUMEN_MEMOIZE not set)"
echo "Expected: ~30 seconds"
echo "Command: /home/user/lumen-lang/target/debug/stream $FIBONACCI_RECURSIVE"
echo ""
time /home/user/lumen-lang/target/debug/stream "$FIBONACCI_RECURSIVE" > /tmp/fib_no_memo.txt 2>&1
echo "First line of output: $(head -1 /tmp/fib_no_memo.txt)"
echo ""

echo "======================================"
echo ""

echo "Test 2: WITH memoization (LUMEN_MEMOIZE=1)"
echo "Expected: <100ms"
echo "Command: LUMEN_MEMOIZE=1 /home/user/lumen-lang/target/debug/stream $FIBONACCI_RECURSIVE"
echo ""
time env LUMEN_MEMOIZE=1 /home/user/lumen-lang/target/debug/stream "$FIBONACCI_RECURSIVE" > /tmp/fib_with_memo.txt 2>&1
echo "First line of output: $(head -1 /tmp/fib_with_memo.txt)"
echo ""

echo "======================================"
echo "Comparison:"
echo "======================================"
echo "Outputs match: $(diff /tmp/fib_no_memo.txt /tmp/fib_with_memo.txt && echo 'YES' || echo 'NO')"
