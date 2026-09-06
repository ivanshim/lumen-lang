<?php
// Ported from examples/lumen/libraries/test_string_transform.lm by scripts/port_examples.py; edit the Lumen original, not this file.
function substring($s, $from_start, $to_end) {
    $index = $from_start;
    $out = "";
    while ($index < $to_end) {
        $out = $out . $s[$index];
        $index = $index + 1;
    }
    return $out;
}

function substring_end($s, $from_here) {
    return substring($s, $from_here, count($s));
}

function char_to_upper($c) {
    $o = ord($c);
    if ($o >= ord("a") && $o <= ord("z")) {
        return chr($o - 32);
    } else {
        return $c;
    }
}

function char_to_lower($c) {
    $o = ord($c);
    if ($o >= ord("A") && $o <= ord("Z")) {
        return chr($o + 32);
    } else {
        return $c;
    }
}

function string_to_upper($s) {
    $result = "";
    $i = 0;
    while ($i < count($s)) {
        $result = $result . char_to_upper($s[$i]);
        $i = $i + 1;
    }
    return $result;
}

function string_to_lower($s) {
    $result = "";
    $i = 0;
    while ($i < count($s)) {
        $result = $result . char_to_lower($s[$i]);
        $i = $i + 1;
    }
    return $result;
}

function reverse_characters($s) {
    $result = "";
    $index = count($s) - 1;
    while ($index >= 0) {
        $result = $result . $s[$index];
        $index = $index - 1;
    }
    return $result;
}

print("=== String Case Transformation ===\n");
print("\n");
print("Uppercase conversion:\n");
print(("  string_to_upper('hello'): " . string_to_upper("hello")) . "\n");
print(("  string_to_upper('world'): " . string_to_upper("world")) . "\n");
print(("  string_to_upper('Hello123'): " . string_to_upper("Hello123")) . "\n");
print("\n");
print("Lowercase conversion:\n");
print(("  string_to_lower('HELLO'): " . string_to_lower("HELLO")) . "\n");
print(("  string_to_lower('WORLD'): " . string_to_lower("WORLD")) . "\n");
print(("  string_to_lower('Hello123'): " . string_to_lower("Hello123")) . "\n");
print("\n");
print("Single character transformations:\n");
print(("  char_to_upper('a'): " . char_to_upper("a")) . "\n");
print(("  char_to_upper('z'): " . char_to_upper("z")) . "\n");
print(("  char_to_lower('A'): " . char_to_lower("A")) . "\n");
print(("  char_to_lower('Z'): " . char_to_lower("Z")) . "\n");
print(("  char_to_upper('5'): " . char_to_upper("5")) . "\n");
print(("  char_to_lower('5'): " . char_to_lower("5")) . "\n");
print("\n");
print("String reversal:\n");
print(("  reverse_characters('abc'): " . reverse_characters("abc")) . "\n");
print(("  reverse_characters('hello'): " . reverse_characters("hello")) . "\n");
print(("  reverse_characters('racecar'): " . reverse_characters("racecar")) . "\n");
print(("  reverse_characters('12345'): " . reverse_characters("12345")) . "\n");
print("\n");
print("=== Practical Example: Title Case ===\n");
function to_title_case($s) {
    if (count($s) == 0) {
        return $s;
    }
    return char_to_upper($s[0]) . substring_end(string_to_lower($s), 1);
}

$words = ["hello", "world", "lumen", "PROGRAMMING"];
$i = 0;
while ($i < count($words)) {
    $word = $words[$i];
    print(("  " . $word . " -> " . to_title_case($word)) . "\n");
    $i = $i + 1;
}
print("\n");
print("=== Practical Example: Palindrome Checker ===\n");
function is_palindrome($s) {
    $normalized = string_to_lower($s);
    return $normalized == reverse_characters($normalized);
}

$test_words = ["racecar", "hello", "madam", "world", "level"];
$i = 0;
while ($i < count($test_words)) {
    $word = $test_words[$i];
    $is_pal = is_palindrome($word);
    print(("  '" . $word . "' is palindrome: " . strval($is_pal)) . "\n");
    $i = $i + 1;
}
