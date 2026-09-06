<?php
// Ported from examples/lumen/constructs/return_explicit.lm by scripts/port_examples.py; edit the Lumen original, not this file.
function absolute($x) {
    if ($x < 0) {
        return -$x;
    }
    return $x;
}

function safe_divide($a, $b) {
    if ($b == 0) {
        return null;
    }
    return $a / $b;
}

function find_first_even($a, $b, $c) {
    if ($a % 2 == 0) {
        return $a;
    }
    if ($b % 2 == 0) {
        return $b;
    }
    return $c;
}

print("Test: Explicit Returns\n");
print("absolute(-5):\n");
print(absolute(-5) . "\n");
print(absolute(5) . "\n");
print("safe_divide(10, 2):\n");
print(safe_divide(10, 2) . "\n");
print("safe_divide(10, 0):\n");
print(safe_divide(10, 0) . "\n");
print("find_first_even(1, 2, 3):\n");
print(find_first_even(1, 2, 3) . "\n");
print("find_first_even(2, 5, 7):\n");
print(find_first_even(2, 5, 7) . "\n");
