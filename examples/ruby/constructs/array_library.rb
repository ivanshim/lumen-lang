# Ported from examples/lumen/constructs/array_library.lm by scripts/port_examples.py; edit the Lumen original, not this file.
a = [1, 2, 3]
b = [4, 5]
both = array_concat(a, b)
puts(both)
puts(array_slice(both, 1, 4))
puts(array_index_of(b, 5))
puts(array_index_of(b, 9))
puts(array_contains(a, 2))
puts(array_contains(a, 7))
puts(array_reverse(both))
puts(array_reverse([]).length)
