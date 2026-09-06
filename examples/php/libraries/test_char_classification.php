<?php
// Ported from examples/lumen/libraries/test_char_classification.lm by scripts/port_examples.py; edit the Lumen original, not this file.
function is_ascii($c) {
    return ord($c) < 128;
}

function is_digit($c) {
    $o = ord($c);
    return $o >= ord("0") && $o <= ord("9");
}

function is_alpha($c) {
    $o = ord($c);
    return ($o >= ord("A") && $o <= ord("Z")) || ($o >= ord("a") && $o <= ord("z"));
}

function is_alnum($c) {
    return is_alpha($c) || is_digit($c);
}

print("=== Character Classification ===\n");
print("\n");
print("ASCII characters:\n");
print(("  is_ascii('A'): " . strval(is_ascii("A"))) . "\n");
print(("  is_ascii('5'): " . strval(is_ascii("5"))) . "\n");
print(("  is_ascii(' '): " . strval(is_ascii(" "))) . "\n");
print("\n");
print("Digit detection:\n");
print(("  is_digit('0'): " . strval(is_digit("0"))) . "\n");
print(("  is_digit('9'): " . strval(is_digit("9"))) . "\n");
print(("  is_digit('a'): " . strval(is_digit("a"))) . "\n");
print(("  is_digit('A'): " . strval(is_digit("A"))) . "\n");
print("\n");
print("Alphabetic detection:\n");
print(("  is_alpha('a'): " . strval(is_alpha("a"))) . "\n");
print(("  is_alpha('Z'): " . strval(is_alpha("Z"))) . "\n");
print(("  is_alpha('5'): " . strval(is_alpha("5"))) . "\n");
print(("  is_alpha('!'): " . strval(is_alpha("!"))) . "\n");
print("\n");
print("Alphanumeric detection:\n");
print(("  is_alnum('a'): " . strval(is_alnum("a"))) . "\n");
print(("  is_alnum('Z'): " . strval(is_alnum("Z"))) . "\n");
print(("  is_alnum('5'): " . strval(is_alnum("5"))) . "\n");
print(("  is_alnum('!'): " . strval(is_alnum("!"))) . "\n");
print(("  is_alnum(' '): " . strval(is_alnum(" "))) . "\n");
print("\n");
print("=== Practical Example: Username Validation ===\n");
function is_valid_username_char($c) {
    return is_alnum($c) || $c == "_" || $c == "-";
}

$test_chars = ["a", "Z", "5", "_", "-", "!", "@"];
$i = 0;
while ($i < count($test_chars)) {
    $c = $test_chars[$i];
    $valid = is_valid_username_char($c);
    print(("  '" . $c . "' is valid username char: " . strval($valid)) . "\n");
    $i = $i + 1;
}
