<?php
// Ported from examples/lumen/constructs/array_library.lm by scripts/port_examples.py; edit the Lumen original, not this file.
$a = [1, 2, 3];
$b = [4, 5];
$both = array_concat($a, $b);
print($both . "\n");
print(array_slice_($both, 1, 4) . "\n");
print(array_index_of($b, 5) . "\n");
print(array_index_of($b, 9) . "\n");
print(array_contains($a, 2) . "\n");
print(array_contains($a, 7) . "\n");
print(array_reverse_($both) . "\n");
print(count(array_reverse_([])) . "\n");
