// Ported from examples/lumen/constructs/scope_if.lm by scripts/port_examples.py; edit the Lumen original, not this file.
let x = 10;
if (true) {
    x = 20;
    console.log(x);
} else {
    x = 30;
    console.log(x);
}
console.log(x);
