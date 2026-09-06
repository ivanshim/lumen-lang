// Ported from examples/lumen/exponentiation_naive.lm by scripts/port_examples.py; edit the Lumen original, not this file.
int main(void) {
    long j;
    long base = 7;
    long exp = 100;
    long mod = 1000000007;
    long iterations = 100;
    puts("Naive exponentiation benchmark");
    printf("base = ");
    printf("%ld\n", base);
    printf("exp  = ");
    printf("%ld\n", exp);
    printf("mod  = ");
    printf("%ld\n", mod);
    printf("iterations = ");
    printf("%ld\n", iterations);
    puts("");
    puts("Running naive exponentiation...");
    long result = 0;
    long i = 0;
    while (i < iterations) {
        result = 1;
        j = 0;
        while (j < exp) {
            result = result * base;
            j = j + 1;
        }
        result = result % mod;
        i = i + 1;
    }
    printf("Result: ");
    printf("%ld\n", result);
    puts("Done!");
    return 0;
}
