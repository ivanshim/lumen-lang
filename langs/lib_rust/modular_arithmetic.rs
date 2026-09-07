// The Lumen library file langs/lib_lumen/modular_arithmetic.lm, ported by scripts/port_examples.py; edit the Lumen original, not this file.

fn mod_mult(a: i64, b: i64, m: i64) -> i64 {
    return (a * b) % m;
}

fn mod_pow(base: i64, exp: i64, m: i64) -> i64 {
    let mut result = 1;
    base = base % m;
    while exp > 0 {
        if exp % 2 == 1 {
            result = (result * base) % m;
        }
        exp = exp / 2;
        base = (base * base) % m;
    }
    return result;
}
