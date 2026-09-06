<?php
// Ported from examples/lumen/constructs/until_loop.lm by scripts/port_examples.py; edit the Lumen original, not this file.
print("Until loop ascending (0-9): ");
$i = 0;
while (!($i >= 10)) {
    print($i);
    if ($i < 9) {
        print(", ");
    }
    $i = $i + 1;
}
print("\n");
print("Until loop descending (15-6): ");
$x = 15;
while (!($x <= 5)) {
    print($x);
    if ($x > 6) {
        print(", ");
    }
    $x = $x - 1;
}
print("\n");
