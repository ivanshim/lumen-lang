// Ported from examples/lumen/constructs/let_keyword.lm by scripts/port_examples.py; edit the Lumen original, not this file.
console.log("Test: let and let mut Keywords");
let x = 10;
console.log("let x = 10");
console.log(x);
let y = 5;
console.log("let mut y = 5");
console.log(y);
y = 20;
console.log("After y = 20:");
console.log(y);
x = 100;
console.log("After let x = 100 (shadowing):");
console.log(x);
const result = x + y;
console.log("let result = x + y");
console.log(result);
function test_let() {
    const a = 42;
    let b = 10;
    b = 50;
    return a + b;
}

console.log("test_let():");
console.log(test_let());
