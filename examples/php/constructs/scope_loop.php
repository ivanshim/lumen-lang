<?php
// Ported from examples/lumen/constructs/scope_loop.lm by scripts/port_examples.py; edit the Lumen original, not this file.
$i = 0;
$sum = 0;
while ($i < 5) {
    $sum = $sum + $i;
    print($sum . "\n");
    $i = $i + 1;
}
print($i . "\n");
print($sum . "\n");
