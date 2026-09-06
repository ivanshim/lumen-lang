// Ported from examples/lumen/constructs/for_loop.lm by scripts/port_examples.py; edit the Lumen original, not this file.
int main(void) {
    long i = 0;
    while (i < 10) {
        printf("%ld", i);
        if (i < 9) {
            printf(", ");
        }
        i = i + 1;
    }
    puts("");
    return 0;
}
