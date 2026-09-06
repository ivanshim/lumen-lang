// Ported from examples/lumen/exponentiation_exponent_then_mod.lm by scripts/port_examples.py; edit the Lumen original, not this file.
long mod_pow(long base, long exp, long m) {
    long result = 1;
    base = base % m;
    while (exp > 0) {
        if (exp % 2 == 1) {
            result = (result * base) % m;
        }
        exp = exp / 2;
        base = (base * base) % m;
    }
    return result;
}

int main(void) {
    long base = 7;
    long exp = 100;
    long mod = 1000000007;
    long iterations = 100;
    puts("Fast modular exponentiation benchmark");
    printf("base = ");
    printf("%ld\n", base);
    printf("exp  = ");
    printf("%ld\n", exp);
    printf("mod  = ");
    printf("%ld\n", mod);
    printf("iterations = ");
    printf("%ld\n", iterations);
    puts("");
    puts("Running mod_pow...");
    long result = 0;
    long i = 0;
    while (i < iterations) {
        result = mod_pow(base, exp, mod);
        i = i + 1;
    }
    printf("Result: ");
    printf("%ld\n", result);
    puts("Done!");
    return 0;
}
