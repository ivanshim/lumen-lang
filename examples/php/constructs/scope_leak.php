<?php
// Ported from examples/lumen/constructs/scope_leak.lm by scripts/port_examples.py; edit the Lumen original, not this file.
$y = 100;
print($y . "\n");
if (true) {
    $y = 50;
    print($y . "\n");
}
print($y . "\n");
