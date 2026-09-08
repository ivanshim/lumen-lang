// Ported from examples/lumen/constructs/unicode_identifiers.lm by scripts/port_examples.py; edit the Lumen original, not this file.
long größe(long x) {
    return x + 1;
}

int main(void) {
    long café = 3;
    double π = 3.14159;
    long 数 = café * 2;
    printf("%ld\n", café);
    printf("%f\n", π);
    printf("%ld\n", 数);
    printf("%ld\n", größe(数));
    return 0;
}
