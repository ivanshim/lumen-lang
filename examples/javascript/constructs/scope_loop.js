// Ported from examples/lumen/constructs/scope_loop.lm by scripts/port_examples.py; edit the Lumen original, not this file.
let i = 0;
let sum = 0;
while (i < 5) {
    sum = sum + i;
    console.log(sum);
    i = i + 1;
}
console.log(i);
console.log(sum);
