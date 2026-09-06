<?php
// Ported from examples/lumen/constructs/let_keyword.lm by scripts/port_examples.py; edit the Lumen original, not this file.
print("Test: let and let mut Keywords\n");
$x = 10;
print("let x = 10\n");
print($x . "\n");
$y = 5;
print("let mut y = 5\n");
print($y . "\n");
$y = 20;
print("After y = 20:\n");
print($y . "\n");
$x = 100;
print("After let x = 100 (shadowing):\n");
print($x . "\n");
$result = $x + $y;
print("let result = x + y\n");
print($result . "\n");
function test_let() {
    $a = 42;
    $b = 10;
    $b = 50;
    return $a + $b;
}

print("test_let():\n");
print(test_let() . "\n");
