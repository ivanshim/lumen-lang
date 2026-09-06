<?php
// Ported from examples/lumen/string_library_demo.lm by scripts/port_examples.py; edit the Lumen original, not this file.
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

function substring_start($s, $to_here) {
    return substring($s, 0, $to_here);
}

function starts_with($s, $prefix) {
    return count($prefix) <= count($s) && substring($s, 0, count($prefix)) == $prefix;
}

function ends_with($s, $suffix) {
    return count($suffix) <= count($s) && substring($s, count($s) - count($suffix), count($s)) == $suffix;
}

function repeat_string($s, $repetitions) {
    $out = "";
    $i = 0;
    while ($i < $repetitions) {
        $out = $out . $s;
        $i = $i + 1;
    }
    return $out;
}

function join_strings($arr, $separator) {
    $out = "";
    $n = count($arr);
    $i = 0;
    while ($i < $n) {
        if ($i > 0) {
            $out = $out . $separator;
        }
        $out = $out . $arr[$i];
        $i = $i + 1;
    }
    return $out;
}

function index_of($s, $needle) {
    $n = count($needle);
    $i = 0;
    while ($i + $n <= count($s)) {
        if (substring($s, $i, $i + $n) == $needle) {
            return $i;
        }
        $i = $i + 1;
    }
    return -1;
}

function has_substring($s, $needle) {
    return index_of($s, $needle) >= 0;
}

print("=== String Library Examples ===\n");
print("\n");
$text = "Hello World";
print("Original: ");
print($text . "\n");
print("substring(text, 0, 5): ");
print(substring($text, 0, 5) . "\n");
print("substring(text, 6, 11): ");
print(substring($text, 6, 11) . "\n");
print("\n");
print("substring_end(text, 6): ");
print(substring_end($text, 6) . "\n");
print("\n");
print("substring_start(text, 5): ");
print(substring_start($text, 5) . "\n");
print("\n");
print("starts_with('Hello World', 'Hello'): ");
print(starts_with($text, "Hello") . "\n");
print("starts_with('Hello World', 'World'): ");
print(starts_with($text, "World") . "\n");
print("\n");
print("ends_with('Hello World', 'World'): ");
print(ends_with($text, "World") . "\n");
print("ends_with('Hello World', 'Hello'): ");
print(ends_with($text, "Hello") . "\n");
print("\n");
print("repeat_string('Ha', 5): ");
print(repeat_string("Ha", 5) . "\n");
print("repeat_string('-=', 10): ");
print(repeat_string("-=", 10) . "\n");
print("\n");
$fruits = ["apple", "banana", "cherry"];
print("join_strings(['apple', 'banana', 'cherry'], ', '): ");
print(join_strings($fruits, ", ") . "\n");
print("join_strings(['apple', 'banana', 'cherry'], ' | '): ");
print(join_strings($fruits, " | ") . "\n");
print("\n");
$sentence = "The quick brown fox jumps over the lazy dog";
print("index_of('The quick brown fox...', 'fox'): ");
print(index_of($sentence, "fox") . "\n");
print("index_of('The quick brown fox...', 'cat'): ");
print(index_of($sentence, "cat") . "\n");
print("\n");
print("has_substring('The quick brown fox...', 'quick'): ");
print(has_substring($sentence, "quick") . "\n");
print("has_substring('The quick brown fox...', 'slow'): ");
print(has_substring($sentence, "slow") . "\n");
print("\n");
print("=== Practical Example ===\n");
$name = "Lumen";
$version = "1.0";
$description = "A minimal language";
$separator = repeat_string("-", 40);
print($separator . "\n");
$info = "Project: " . $name;
print($info . "\n");
$info2 = "Version: " . $version;
print($info2 . "\n");
$info3 = "Description: " . $description;
print($info3 . "\n");
print($separator . "\n");
