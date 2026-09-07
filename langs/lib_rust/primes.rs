// The Lumen library file langs/lib_lumen/primes.lm, ported by scripts/port_examples.py; edit the Lumen original, not this file.

fn next_prime(n: i64) {
    let mut k = n + 1;
    while true {
        if is_prime(k) {
            k;
        }
        k = k + 1;
    }
}
