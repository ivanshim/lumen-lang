// Ported from examples/lumen/constructs/scope_loop.lm by scripts/port_examples.py; edit the Lumen original, not this file.
int main(void) {
    long i = 0;
    long sum = 0;
    while (i < 5) {
        sum = sum + i;
        printf("%ld\n", sum);
        i = i + 1;
    }
    printf("%ld\n", i);
    printf("%ld\n", sum);
    return 0;
}
