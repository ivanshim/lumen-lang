# The Lumen library file langs/lib_lumen/array.lm, ported by scripts/port_examples.py; edit the Lumen original, not this file.

def array_concat(a, b):
    out = [];
    i = 0;
    while i < len(a):
        out.append(a[i]);
        i = i + 1;
    i = 0;
    while i < len(b):
        out.append(b[i]);
        i = i + 1;
    return out;

def array_slice(a, start, stop):
    out = [];
    i = start;
    while i < stop:
        out.append(a[i]);
        i = i + 1;
    return out;

def array_index_of(a, x):
    i = 0;
    while i < len(a):
        if a[i] == x:
            return i;
        i = i + 1;
    return -1;

def array_contains(a, x):
    return array_index_of(a, x) >= 0;

def array_reverse(a):
    out = [];
    i = len(a);
    while i > 0:
        i = i - 1;
        out.append(a[i]);
    return out;
