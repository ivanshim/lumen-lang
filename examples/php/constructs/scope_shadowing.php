<?php
// Ported from examples/lumen/constructs/scope_shadowing.lm by scripts/port_examples.py; edit the Lumen original, not this file.
print(1 . "\n");
$x = 10;
print($x . "\n");
if (true) {
    $x = 20;
    print($x . "\n");
}
print($x . "\n");
