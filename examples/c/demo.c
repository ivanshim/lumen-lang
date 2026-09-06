#include <stdio.h>
#include <stdbool.h>

int main(void) {
    printf("%d\n", 1 + 2 * 3);

    int x = 0;
    int y = 5;

    if (x < y && y == 5) {
        printf("%d\n", 100);
    } else if (x == y) {
        printf("%d\n", 150);
    } else {
        printf("%d\n", 200);
    }

    int i = 0;
    int sum = 0;

    while (i < 10) {
        if (i == 5) {
            i = i + 1;
            continue;
        }

        if (i == 8) {
            break;
        }

        sum = sum + i;
        printf("%d\n", sum);
        i = i + 1;
    }

    printf("%d\n", sum);
    printf("%d\n", -10 + 3);
    printf("%d\n", 0x1F);
    printf("%d and %s\n", 2, "two");

    bool ok = true;
    if (ok) {
        puts("ok");
    }

    /* A block comment. */
    puts("done");
    return 0;
}
