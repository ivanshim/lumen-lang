// Ported from examples/lumen/constructs/demo.lm by scripts/port_examples.py; edit the Lumen original, not this file.
int main(void) {
    printf("%ld\n", 1 + 2 * 3);
    long x = 0;
    long y = 5;
    if (x < y && y == 5) {
        printf("%ld\n", 100);
    } else {
        printf("%ld\n", 200);
    }
    long i = 0;
    long sum = 0;
    while (i < 10) {
        if (i == 5) {
            i = i + 1;
            continue;
        }
        if (i == 8) {
            break;
        }
        sum = sum + i;
        printf("%ld\n", sum);
        i = i + 1;
    }
    printf("%ld\n", sum);
    printf("%d\n", true);
    printf("%d\n", false);
    printf("%d\n", !false);
    printf("%ld\n", -10 + 3);
    return 0;
}
