<?php
// Ported from examples/lumen/exponentiation_naive.lm by scripts/port_examples.py; edit the Lumen original, not this file.
$base = 7;
$exp = 100;
$mod = 1000000007;
$iterations = 100;
print("Naive exponentiation benchmark\n");
print("base = ");
print($base . "\n");
print("exp  = ");
print($exp . "\n");
print("mod  = ");
print($mod . "\n");
print("iterations = ");
print($iterations . "\n");
print("\n");
print("Running naive exponentiation...\n");
$result = 0;
$i = 0;
while ($i < $iterations) {
    $result = 1;
    $j = 0;
    while ($j < $exp) {
        $result = $result * $base;
        $j = $j + 1;
    }
    $result = $result % $mod;
    $i = $i + 1;
}
print("Result: ");
print($result . "\n");
print("Done!\n");
