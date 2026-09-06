<?php
// Ported from examples/lumen/constructs/for_loop.lm by scripts/port_examples.py; edit the Lumen original, not this file.
$i = 0;
while ($i < 10) {
    print($i);
    if ($i < 9) {
        print(", ");
    }
    $i = $i + 1;
}
print("\n");
