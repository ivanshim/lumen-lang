<?php
// Ported from examples/lumen/constructs/operators_complete.lm by scripts/port_examples.py; edit the Lumen original, not this file.
print("Test: Complete Operators\n");
print("Logical OR:\n");
print((true || false) . "\n");
print((false || true) . "\n");
print((false || false) . "\n");
print((true || true) . "\n");
$x = 5;
if ($x < 3 || $x > 4) {
    print("x is either less than 3 or greater than 4\n");
} else {
    print("x is between 3 and 4\n");
}
print("Greater-than-or-equal:\n");
print((10 >= 5) . "\n");
print((5 >= 5) . "\n");
print((3 >= 5) . "\n");
if ($x >= 5) {
    print("x is greater than or equal to 5\n");
} else {
    print("x is less than 5\n");
}
print("Exponentiation:\n");
print(2 ** 3 . "\n");
print(5 ** 2 . "\n");
print(10 ** 0 . "\n");
$result = 2 ** 3 + 1;
print("2 ** 3 + 1 = \n");
print($result . "\n");
function power($base, $exp) {
    return $base ** $exp;
}

print("power(3, 4):\n");
print(power(3, 4) . "\n");
print("Precedence test: 2 + 3 ** 2\n");
print(2 + 3 ** 2 . "\n");
print("Precedence test: 10 / 2 ** 2\n");
print(10 / 2 ** 2 . "\n");
