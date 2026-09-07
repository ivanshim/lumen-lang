// The Lumen library file langs/lib_lumen/primes.lm, ported by scripts/port_examples.py; edit the Lumen original, not this file.

func next_prime(n: Int) {
    var k = n + 1
    while true {
        if is_prime(n: k) {
            k
        }
        k = k + 1
    }
}
