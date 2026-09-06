<?php
// Ported from examples/lumen/constructs/string_operations.lm by scripts/port_examples.py; edit the Lumen original, not this file.
function char_at_or_null($s, $index) {
    if ($index < 0 || $index >= count($s)) {
        return null;
    }
    return $s[$index];
}

print("=== String Operations Test ===\n");
$str1 = "Hello";
$str2 = " World";
$result1 = $str1 . $str2;
print("Period operator (string . string): ");
print($result1 . "\n");
$num = 42;
$result2 = "Answer: " . $num;
print("Period operator with number coercion: ");
print($result2 . "\n");
$x = 10;
$y = 20;
$result3 = "Sum: " . ($x + $y);
print("Period operator with expression: ");
print($result3 . "\n");
$test_str = "Hello";
$str_len = strlen($test_str);
print("len('Hello'): ");
print($str_len . "\n");
$utf8_str = "abc123";
$utf8_len = strlen($utf8_str);
print("len('abc123'): ");
print($utf8_len . "\n");
$empty_str = "";
$empty_len = strlen($empty_str);
print("len(''): ");
print($empty_len . "\n");
$arr = [1, 2, 3, 4, 5];
$arr_len = count($arr);
print("len([1,2,3,4,5]): ");
print($arr_len . "\n");
$text = "Lumen";
$ch0 = $text[0];
print("char_at('Lumen', 0): ");
print($ch0 . "\n");
$ch2 = $text[2];
print("char_at('Lumen', 2): ");
print($ch2 . "\n");
$ch4 = $text[4];
print("char_at('Lumen', 4): ");
print($ch4 . "\n");
print("\n");
print("=== Testing char_at_or_null (permissive wrapper) ===\n");
$ch_valid = char_at_or_null($text, 1);
print("char_at_or_null('Lumen', 1): ");
print($ch_valid . "\n");
$ch_oob = char_at_or_null($text, 10);
print("char_at_or_null('Lumen', 10) [out of bounds]: ");
print($ch_oob . "\n");
$ch_neg = char_at_or_null($text, -1);
print("char_at_or_null('Lumen', -1) [negative]: ");
print($ch_neg . "\n");
$ch_edge = char_at_or_null($text, 5);
print("char_at_or_null('Lumen', 5) [at length]: ");
print($ch_edge . "\n");
$word = "Test";
$length = strlen($word);
$first_char = $word[0];
$result10 = "Word: " . $word . ", Length: " . $length . ", First: " . $first_char;
print("Combined operations: ");
print($result10 . "\n");
print("Done!\n");
