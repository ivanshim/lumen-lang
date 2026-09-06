<?php
// Ported from examples/lumen/constructs/functions_basic.lm by scripts/port_examples.py; edit the Lumen original, not this file.
function square($x) {
    return $x * $x;
}

function add($a, $b) {
    return $a + $b;
}

function greet($name) {
    return "Hello, " + $name;
}

print("Test: Basic Functions\n");
print(square(5) . "\n");
print(add(10, 20) . "\n");
print(greet("Lumen") . "\n");
function get_constant() {
    return 42;
}

print(get_constant() . "\n");
function compute($x, $y) {
    $sum = $x + $y;
    $product = $x * $y;
    return $sum + $product;
}

print(compute(3, 4) . "\n");
