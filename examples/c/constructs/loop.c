// Ported from examples/lumen/constructs/loop.lm by scripts/port_examples.py; edit the Lumen original, not this file.
int main(void) {
    long x = 0;
    while (x < 10) {
        printf("%ld", x);
        if (x < 9) {
            printf(", ");
        }
        x = x + 1;
    }
    puts("");
    return 0;
}
