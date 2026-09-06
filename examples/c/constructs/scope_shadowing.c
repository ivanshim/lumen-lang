// Ported from examples/lumen/constructs/scope_shadowing.lm by scripts/port_examples.py; edit the Lumen original, not this file.
int main(void) {
    printf("%ld\n", 1);
    long x = 10;
    printf("%ld\n", x);
    if (true) {
        x = 20;
        printf("%ld\n", x);
    }
    printf("%ld\n", x);
    return 0;
}
