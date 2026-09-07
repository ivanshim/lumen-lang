// The Lumen library file lib_lumen/modular_arithmetic.lm, ported by scripts/port_examples.py; edit the Lumen original, not this file.

func mod_mult(a: Int, b: Int, m: Int) -> Int {
    return (a * b) % m
}

func mod_pow(base: Int, exp: Int, m: Int) -> Int {
    var result = 1
    base = base % m
    while exp > 0 {
        if exp % 2 == 1 {
            result = (result * base) % m
        }
        exp = exp / 2
        base = (base * base) % m
    }
    return result
}
