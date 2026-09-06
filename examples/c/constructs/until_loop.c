// Ported from examples/lumen/constructs/until_loop.lm by scripts/port_examples.py; edit the Lumen original, not this file.
int main(void) {
    printf("Until loop ascending (0-9): ");
    long i = 0;
    while (!(i >= 10)) {
        printf("%ld", i);
        if (i < 9) {
            printf(", ");
        }
        i = i + 1;
    }
    puts("");
    printf("Until loop descending (15-6): ");
    long x = 15;
    while (!(x <= 5)) {
        printf("%ld", x);
        if (x > 6) {
            printf(", ");
        }
        x = x - 1;
    }
    puts("");
    return 0;
}
