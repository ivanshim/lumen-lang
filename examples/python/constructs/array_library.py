# Ported from examples/lumen/constructs/array_library.lm by scripts/port_examples.py; edit the Lumen original, not this file.
a = [1, 2, 3]
b = [4, 5]
both = array_concat(a= a, b= b)
print(both)
print(array_slice(a= both, start= 1, stop= 4))
print(array_index_of(a= b, x= 5))
print(array_index_of(a= b, x= 9))
print(array_contains(a= a, x= 2))
print(array_contains(a= a, x= 7))
print(array_reverse(a= both))
print(len(array_reverse(a= [])))
