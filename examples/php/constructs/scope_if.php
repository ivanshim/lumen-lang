<?php
// Ported from examples/lumen/constructs/scope_if.lm by scripts/port_examples.py; edit the Lumen original, not this file.
$x = 10;
if (true) {
    $x = 20;
    print($x . "\n");
} else {
    $x = 30;
    print($x . "\n");
}
print($x . "\n");
