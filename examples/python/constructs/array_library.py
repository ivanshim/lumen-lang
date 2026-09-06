# Ported from examples/lumen/constructs/array_library.lm by scripts/port_examples.py; edit the Lumen original, not this file.
def array_concat(a, b):
    out = []
    i = 0
    while i < len(a):
        out.append(a[i])
        i = i + 1
    i = 0
    while i < len(b):
        out.append(b[i])
        i = i + 1
    return out

def array_slice(a, start, stop):
    out = []
    i = start
    while i < stop:
        out.append(a[i])
        i = i + 1
    return out

def array_index_of(a, x):
    i = 0
    while i < len(a):
        if a[i] == x:
            return i
        i = i + 1
    return -1

def array_contains(a, x):
    return array_index_of(a, x) >= 0

def array_reverse(a):
    out = []
    i = len(a)
    while i > 0:
        i = i - 1
        out.append(a[i])
    return out

a = [1, 2, 3]
b = [4, 5]
both = array_concat(a, b)
print(both)
print(array_slice(both, 1, 4))
print(array_index_of(b, 5))
print(array_index_of(b, 9))
print(array_contains(a, 2))
print(array_contains(a, 7))
print(array_reverse(both))
print(len(array_reverse([])))
