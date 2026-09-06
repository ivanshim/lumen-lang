// Ported from examples/lumen/constructs/loop.lm by scripts/port_examples.py; edit the Lumen original, not this file.
let x = 0;
while (x < 10) {
    process.stdout.write(String(x));
    if (x < 9) {
        process.stdout.write(", ");
    }
    x = x + 1;
}
console.log("");
