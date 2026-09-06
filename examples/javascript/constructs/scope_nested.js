// Ported from examples/lumen/constructs/scope_nested.lm by scripts/port_examples.py; edit the Lumen original, not this file.
let x = 1;
if (true) {
    x = 2;
    if (true) {
        x = 3;
        console.log(x);
    }
    console.log(x);
}
console.log(x);
