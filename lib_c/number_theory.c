// The Lumen library file lib_lumen/number_theory.lm, ported by scripts/port_examples.py; edit the Lumen original, not this file.

long gcd(long a, long b) {
    long t;
    while (b != 0) {
        t = b;
        b = a % b;
        a = t;
    }
    return a;
}

long lcm(long a, long b) {
    return (a * b) / gcd(a, b);
}

bool is_coprime(long a, long b) {
    return gcd(a, b) == 1;
}

bool is_prime(long n) {
    if (n < 2) {
        return false;
    }
    if (n == 2) {
        return true;
    }
    if (n % 2 == 0) {
        return false;
    }
    long i = 3;
    while (i * i <= n) {
        if (n % i == 0) {
            return false;
        }
        i = i + 2;
    }
    return true;
}

long isqrt(long n) {
    long x1;
    if (n == 0) {
        return 0;
    }
    long x = n;
    while (true) {
        x1 = (x + n / x) / 2;
        if (x1 >= x) {
            return x;
        }
        x = x1;
    }
}

bool is_unit(long a, long m) {
    return gcd(a % m, m) == 1;
}
