// Ported from examples/lumen/constructs/scope_leak.lm by scripts/port_examples.py; edit the Lumen original, not this file.
let y = 100;
console.log(y);
if (true) {
    y = 50;
    console.log(y);
}
console.log(y);
