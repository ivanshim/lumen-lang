// Ported from examples/lumen/constructs/array_library.lm by scripts/port_examples.py; edit the Lumen original, not this file.
const a = [1, 2, 3];
const b = [4, 5];
const both = array_concat(a, b);
console.log(both);
console.log(array_slice(both, 1, 4));
console.log(array_index_of(b, 5));
console.log(array_index_of(b, 9));
console.log(array_contains(a, 2));
console.log(array_contains(a, 7));
console.log(array_reverse(both));
console.log(array_reverse([]).length);
