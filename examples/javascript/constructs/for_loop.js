// Ported from examples/lumen/constructs/for_loop.lm by scripts/port_examples.py; edit the Lumen original, not this file.
let i = 0;
while (i < 10) {
    process.stdout.write(String(i));
    if (i < 9) {
        process.stdout.write(", ");
    }
    i = i + 1;
}
console.log("");
