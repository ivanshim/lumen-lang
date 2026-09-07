// The Lumen library file lib_lumen/factorial.lm, ported by scripts/port_examples.py; edit the Lumen original, not this file.

fn factorial(n: i64) -> i64 {
    if n <= 1 {
        return 1;
    } else {
        return n * factorial(n - 1);
    }
}
