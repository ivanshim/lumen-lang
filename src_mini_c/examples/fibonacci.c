a = 0;
b = 1;
count = 0;

while (count < 10) {
    printf(a);
    next = a + b;
    a = b;
    b = next;
    count = count + 1;
}
