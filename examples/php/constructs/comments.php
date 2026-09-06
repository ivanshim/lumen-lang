<?php
// Ported from examples/lumen/constructs/comments.lm by scripts/port_examples.py; edit the Lumen original, not this file.
print("Test: Comments Support\n");
$x = 42;
print($x . "\n");
$result = $x * 2;
print($result . "\n");
function add_numbers($a, $b) {
    return $a + $b;
}

$value = add_numbers(10, 20);
print($value . "\n");
if ($value > 20) {
    print("Value is greater than 20\n");
} else {
    print("Value is 20 or less\n");
}
$counter = 0;
while ($counter < 3) {
    print($counter . "\n");
    $counter = $counter + 1;
}
print("Done testing comments\n");
