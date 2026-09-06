<?php
// Ported from examples/lumen/libraries/test_string_validation.lm by scripts/port_examples.py; edit the Lumen original, not this file.
function is_alpha($c) {
    $o = ord($c);
    return ($o >= ord("A") && $o <= ord("Z")) || ($o >= ord("a") && $o <= ord("z"));
}

function is_alpha_string($s) {
    if (count($s) == 0) {
        return false;
    }
    $i = 0;
    while ($i < count($s)) {
        if (!is_alpha($s[$i])) {
            return false;
        }
        $i = $i + 1;
    }
    return true;
}

print("=== String Content Validation ===\n");
print("\n");
print("Alphabetic string validation:\n");
print(("  is_alpha_string('hello'): " . strval(is_alpha_string("hello"))) . "\n");
print(("  is_alpha_string('WORLD'): " . strval(is_alpha_string("WORLD"))) . "\n");
print(("  is_alpha_string('LuMeN'): " . strval(is_alpha_string("LuMeN"))) . "\n");
print(("  is_alpha_string('hello123'): " . strval(is_alpha_string("hello123"))) . "\n");
print(("  is_alpha_string('hello world'): " . strval(is_alpha_string("hello world"))) . "\n");
print(("  is_alpha_string(''): " . strval(is_alpha_string(""))) . "\n");
print("\n");
print("=== Practical Example: Name Validation ===\n");
function validate_name_input($s) {
    if (count($s) == 0) {
        print(("  '" . $s . "' - INVALID: name cannot be empty") . "\n");
        return false;
    }
    if (!is_alpha_string($s)) {
        print(("  '" . $s . "' - INVALID: name must contain only letters") . "\n");
        return false;
    }
    print(("  '" . $s . "' - VALID name") . "\n");
    return true;
}

$name_inputs = ["Alice", "Bob123", "Charlie", "", "Dave_Smith"];
$i = 0;
while ($i < count($name_inputs)) {
    validate_name_input($name_inputs[$i]);
    $i = $i + 1;
}
