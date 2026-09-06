<?php
// Ported from examples/lumen/constructs/short_circuit.lm by scripts/port_examples.py; edit the Lumen original, not this file.
function is_even($x) {
    print("Checking if \n");
    print($x . "\n");
    print(" is even\n");
    return $x % 2 == 0;
}

function is_positive($x) {
    print("Checking if \n");
    print($x . "\n");
    print(" is positive\n");
    return $x > 0;
}

print("Test: Short-Circuit Evaluation\n");
print("false and is_even(10):\n");
$result = false && is_even(10);
print($result . "\n");
print("true and is_even(10):\n");
$result = true && is_even(10);
print($result . "\n");
print("true or is_positive(5):\n");
$result = true || is_positive(5);
print($result . "\n");
print("false or is_positive(5):\n");
$result = false || is_positive(5);
print($result . "\n");
print("Testing division by zero avoidance:\n");
$x = 0;
if ($x != 0 && 10 / $x > 5) {
    print("Result is greater than 5\n");
} else {
    print("x is zero or result is not greater than 5\n");
}
function safe_check($value) {
    if ($value != null && $value > 10) {
        print("Value is not null and greater than 10\n");
    } else {
        print("Value is null or not greater than 10\n");
    }
}

safe_check(15);
safe_check(5);
