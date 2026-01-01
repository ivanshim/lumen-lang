a = 0;
b = 1;
count = 0;

while ($count < 20) {
    print($a);
    next = $a + $b;
    a = $b;
    b = $next;
    count = $count + 1;
}
