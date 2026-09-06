<?php
function fib($n) {
    $a = 0;
    $b = 1;
    $i = 0;
    while ($i < $n) {
        $c = $a + $b;
        $a = $b;
        $b = $c;
        $i = $i + 1;
    }
    return $a;
}

$i = 0;
while ($i < 10) {
    print(fib($i) . "\n");
    $i = $i + 1;
}
