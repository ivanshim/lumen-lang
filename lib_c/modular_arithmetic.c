// The Lumen library file lib_lumen/modular_arithmetic.lm, ported by scripts/port_examples.py; edit the Lumen original, not this file.

long mod_mult(long a, long b, long m) {
    return (a * b) % m;
}

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
