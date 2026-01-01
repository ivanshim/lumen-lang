printf(1 + 2 * 3);

x = 0;
y = 5;

if (x < y and y == 5) {
    printf(100);
} else {
    printf(200);
}

i = 0;
sum = 0;

while (i < 10) {
    if (i == 5) {
        i = i + 1;
        continue;
    }

    if (i == 8) {
        break;
    }

    sum = sum + i;
    printf(sum);
    i = i + 1;
}

printf(sum);
printf(true);
printf(false);
printf(not false);
printf(-10 + 3);
