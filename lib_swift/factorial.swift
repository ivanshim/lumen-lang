// The Lumen library file lib_lumen/factorial.lm, ported by scripts/port_examples.py; edit the Lumen original, not this file.

func factorial(n: Int) -> Int {
    if n <= 1 {
        return 1
    } else {
        return n * factorial(n: n - 1)
    }
}
