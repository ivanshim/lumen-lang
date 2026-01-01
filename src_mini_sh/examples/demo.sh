echo(1 + 2 * 3);

x = 0;
y = 5;

if ($x < $y and $y == 5) {
    echo(100);
} else {
    echo(200);
}

i = 0;
sum = 0;

while ($i < 10) {
    if ($i == 5) {
        i = $i + 1;
        continue;
    }

    if ($i == 8) {
        break;
    }

    sum = $sum + $i;
    echo($sum);
    i = $i + 1;
}

echo($sum);
echo(true);
echo(false);
echo(not false);
echo(-10 + 3);
