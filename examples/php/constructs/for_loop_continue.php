<?php
// Ported from examples/lumen/constructs/for_loop_continue.lm by scripts/port_examples.py; edit the Lumen original, not this file.
$i = 0;
while ($i < 11) {
    if ($i == 5) {
        $i = $i + 1;
        continue;
    }
    print($i);
    if ($i < 10) {
        print(", ");
    }
    $i = $i + 1;
}
print("\n");
