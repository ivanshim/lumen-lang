#include <stdio.h>

// Two while loops; the second writes on one line.
int main(void) {
    int x = 0;
    while (x < 5) {
        printf("%d\n", x);
        x = x + 1;
    }

    int i = 5;
    while (i < 10) {
        printf("%d ", i);
        i = i + 1;
    }
    printf("\n");
    return 0;
}
