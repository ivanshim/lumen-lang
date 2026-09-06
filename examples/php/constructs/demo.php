<?php
// Ported from examples/lumen/constructs/demo.lm by scripts/port_examples.py; edit the Lumen original, not this file.
print(1 + 2 * 3 . "\n");
$x = 0;
$y = 5;
if ($x < $y && $y == 5) {
    print(100 . "\n");
} else {
    print(200 . "\n");
}
$i = 0;
$sum = 0;
while ($i < 10) {
    if ($i == 5) {
        $i = $i + 1;
        continue;
    }
    if ($i == 8) {
        break;
    }
    $sum = $sum + $i;
    print($sum . "\n");
    $i = $i + 1;
}
print($sum . "\n");
print(true . "\n");
print(false . "\n");
print(!false . "\n");
print(-10 + 3 . "\n");
