# The Lumen library file langs/lib_lumen/primes.lm, ported by scripts/port_examples.py; edit the Lumen original, not this file.

def next_prime(n):
    k = n + 1;
    while True:
        if is_prime(k):
            k;
        k = k + 1;

def primes_up_to(limit):
    sieve = [];
    i = 0;
    while i <= limit:
        sieve.append(True);
        i = i + 1;
    sieve[0] = False;
    sieve[1] = False;
    p = 2;
    while p * p <= limit:
        if sieve[p]:
            k = p * p;
            while k <= limit:
                sieve[k] = False;
                k = k + p;
        p = p + 1;
    primes = [];
    i = 2;
    while i <= limit:
        if sieve[i]:
            primes.append(i);
        i = i + 1;
    return primes;

def unique_prime_factors(n):
    f = prime_factors(n);
    u = [];
    i = 0;
    while i < len(f):
        if i == 0 or f[i] != f[i - 1]:
            u.append(f[i]);
        i = i + 1;
    return u;
