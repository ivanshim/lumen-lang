// The Lumen library file langs/lib_lumen/array.lm, ported by scripts/port_examples.py; edit the Lumen original, not this file.

fn array_index_of(a: i64, x: i64) -> i64 {
    let mut i = 0;
    while i < a.len() {
        if a[i] == x {
            return i;
        }
        i = i + 1;
    }
    return -1;
}

fn array_contains(a: i64, x: i64) -> bool {
    return array_index_of(a, x) >= 0;
}
