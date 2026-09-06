// Ported from examples/lumen/constructs/scope_shadowing.lm by scripts/port_examples.py; edit the Lumen original, not this file.
console.log(1);
let x = 10;
console.log(x);
if (true) {
    x = 20;
    console.log(x);
}
console.log(x);
