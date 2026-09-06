// Ported from examples/lumen/constructs/scope_leak.lm by scripts/port_examples.py; edit the Lumen original, not this file.
int main(void) {
    long y = 100;
    printf("%ld\n", y);
    if (true) {
        y = 50;
        printf("%ld\n", y);
    }
    printf("%ld\n", y);
    return 0;
}
