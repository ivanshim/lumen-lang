// The Lumen library file langs/lib_lumen/primes.lm, ported by scripts/port_examples.py; edit the Lumen original, not this file.

void next_prime(long n) {
    long k = n + 1;
    while (true) {
        if (is_prime(k)) {
            k;
        }
        k = k + 1;
    }
}
