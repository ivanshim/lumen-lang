// Ported from examples/lumen/constructs/scope_nested.lm by scripts/port_examples.py; edit the Lumen original, not this file.
int main(void) {
    long x = 1;
    if (true) {
        x = 2;
        if (true) {
            x = 3;
            printf("%ld\n", x);
        }
        printf("%ld\n", x);
    }
    printf("%ld\n", x);
    return 0;
}
