// The Lumen library file langs/lib_lumen/array.lm, ported by scripts/port_examples.py; edit the Lumen original, not this file.

func array_index_of(a: Int, x: Int) -> Int {
    var i = 0
    while i < a.count {
        if a[i] == x {
            return i
        }
        i = i + 1
    }
    return -1
}

func array_contains(a: Int, x: Int) -> Bool {
    return array_index_of(a: a, x: x) >= 0
}
