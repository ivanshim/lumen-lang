<?php
// Statements end with semicolons; variables carry a dollar sign.
print(1 + 2 * 3);

$x = 0;
$y = 5;

if ($x < $y && $y == 5) {
    print(100);
} elseif ($x == $y) {
    print(150);
} else {
    print(200);
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
    print($sum);
    $i = $i + 1;
}

print($sum);
print(TRUE);
print(False);
print(!false);
print(-10 + 3);
print("sum is " . $sum);
print('single quotes keep \n literal');
# Hash comments work too.
/* And block
   comments. */
print(0x1F);
