// The Lumen library file lib_lumen/array.lm, ported by scripts/port_examples.py; edit the Lumen original, not this file.

long array_index_of(long a, long x) {
    long i = 0;
    while (i < strlen(a)) {
        if (a[i] == x) {
            return i;
        }
        i = i + 1;
    }
    return -1;
}

bool array_contains(long a, long x) {
    return array_index_of(a, x) >= 0;
}
