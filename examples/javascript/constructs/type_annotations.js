// Ported from examples/lumen/constructs/type_annotations.lm by scripts/port_examples.py; edit the Lumen original, not this file.
console.log("Test: Type Annotations on Variables");
const x = 42;
console.log("let x: number = 42");
console.log(x);
const message = "Hello, World";
console.log("let message: string = ");
console.log(message);
const flag = true;
console.log("let flag: boolean = true");
console.log(flag);
const empty = null;
console.log("let empty: null = null");
console.log(empty);
function add(a, b) {
    return a + b;
}

console.log("add(5, 3):");
console.log(add(5, 3));
function greet(name) {
    return "Hello, " + name;
}

console.log("greet(\"Alice\"):");
console.log(greet("Alice"));
function process(x, y) {
    const result = x * 2 + y;
    return result;
}

console.log("process(10, 5):");
console.log(process(10, 5));
