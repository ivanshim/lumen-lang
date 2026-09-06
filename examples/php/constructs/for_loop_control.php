<?php
// Ported from examples/lumen/constructs/for_loop_control.lm by scripts/port_examples.py; edit the Lumen original, not this file.
$i = 0;
while ($i < 15) {
    if ($i == 10) {
        break;
    }
    print($i);
    if ($i < 9) {
        print(", ");
    }
    $i = $i + 1;
}
print("\n");
