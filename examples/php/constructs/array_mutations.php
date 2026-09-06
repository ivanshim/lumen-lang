<?php
// Ported from examples/lumen/constructs/array_mutations.lm by scripts/port_examples.py; edit the Lumen original, not this file.
print("=== Array Features Test ===\n");
$arr = [10, 20, 30];
print("Array: ");
print($arr . "\n");
print("arr[0] = ");
print($arr[0] . "\n");
$arr2 = [1, 2, 3];
$arr2[1] = 999;
print("After arr2[1]=999: ");
print($arr2 . "\n");
$arr3 = [];
array_push($arr3, 100);
array_push($arr3, 200);
print("After push: ");
print($arr3 . "\n");
print("Done!\n");
