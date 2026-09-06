// Ported from examples/lumen/constructs/array_library.lm by scripts/port_examples.py; edit the Lumen original, not this file.
function array_concat(a, b) {
    const out = [];
    let i = 0;
    while (i < a.length) {
        out.push(a[i]);
        i = i + 1;
    }
    i = 0;
    while (i < b.length) {
        out.push(b[i]);
        i = i + 1;
    }
    return out;
}

function array_slice(a, start, stop) {
    const out = [];
    let i = start;
    while (i < stop) {
        out.push(a[i]);
        i = i + 1;
    }
    return out;
}

function array_index_of(a, x) {
    let i = 0;
    while (i < a.length) {
        if (a[i] === x) {
            return i;
        }
        i = i + 1;
    }
    return -1;
}

function array_contains(a, x) {
    return array_index_of(a, x) >= 0;
}

function array_reverse(a) {
    const out = [];
    let i = a.length;
    while (i > 0) {
        i = i - 1;
        out.push(a[i]);
    }
    return out;
}

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
