// The Lumen library file lib_lumen/primes.lm, ported by scripts/port_examples.py; edit the Lumen original, not this file.

function next_prime(n) {
    let k = n + 1;
    while (true) {
        if (is_prime(k)) {
            k;
        }
        k = k + 1;
    }
}

function primes_up_to(limit) {
    let k;
    const sieve = [];
    let i = 0;
    while (i <= limit) {
        sieve.push(true);
        i = i + 1;
    }
    sieve[0] = false;
    sieve[1] = false;
    let p = 2;
    while (p * p <= limit) {
        if (sieve[p]) {
            k = p * p;
            while (k <= limit) {
                sieve[k] = false;
                k = k + p;
            }
        }
        p = p + 1;
    }
    const primes = [];
    i = 2;
    while (i <= limit) {
        if (sieve[i]) {
            primes.push(i);
        }
        i = i + 1;
    }
    return primes;
}
