<?php
// Ported from examples/lumen/constructs/none_type.lm by scripts/port_examples.py; edit the Lumen original, not this file.
function no_return() {
    print("This function returns null implicitly\n");
}

function explicit_null() {
    print("Returning null explicitly\n");
    return null;
}

function conditional_null($x) {
    if ($x < 0) {
        return null;
    } else {
        return $x * 2;
    }
}

print("Test: null Type\n");
print("Calling no_return():\n");
$result1 = no_return();
print($result1 . "\n");
print("Calling explicit_null():\n");
$result2 = explicit_null();
print($result2 . "\n");
print("conditional_null(5):\n");
print(conditional_null(5) . "\n");
print("conditional_null(-3):\n");
print(conditional_null(-3) . "\n");
$x = null;
print("let x = null:\n");
print($x . "\n");
function check_value($val) {
    if ($val == null) {
        print("Value is null\n");
    } else {
        print("Value is not null\n");
    }
}

check_value(null);
check_value(42);
check_value("hello");
