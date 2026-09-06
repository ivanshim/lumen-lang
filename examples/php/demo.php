<?php
// Statements end with semicolons; variables carry a dollar sign.
print(1 + 2 * 3 . "\n");

$x = 0;
$y = 5;

if ($x < $y && $y == 5) {
    print(100 . "\n");
} elseif ($x == $y) {
    print(150 . "\n");
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
print(TRUE . "\n");
print(False . "\n");
print(!false . "\n");
print(-10 + 3 . "\n");
print("sum is " . $sum . "\n");
print('single quotes keep \n literal' . "\n");
# Hash comments work too.
/* And block
   comments. */
print(0x1F . "\n");
