// Ported from examples/lumen/constructs/scope_if.lm by scripts/port_examples.py; edit the Lumen original, not this file.
int main(void) {
    long x = 10;
    if (true) {
        x = 20;
        printf("%ld\n", x);
    } else {
        x = 30;
        printf("%ld\n", x);
    }
    printf("%ld\n", x);
    return 0;
}
