// Ported from examples/lumen/constructs/functions_basic.lm by scripts/port_examples.py; edit the Lumen original, not this file.
function square(x) {
    return x * x;
}

function add(a, b) {
    return a + b;
}

function greet(name) {
    return "Hello, " + name;
}

console.log("Test: Basic Functions");
console.log(square(5));
console.log(add(10, 20));
console.log(greet("Lumen"));
function get_constant() {
    return 42;
}

console.log(get_constant());
function compute(x, y) {
    const sum = x + y;
    const product = x * y;
    return sum + product;
}

console.log(compute(3, 4));
