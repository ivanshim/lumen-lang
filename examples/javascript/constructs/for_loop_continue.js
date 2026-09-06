// Ported from examples/lumen/constructs/for_loop_continue.lm by scripts/port_examples.py; edit the Lumen original, not this file.
let i = 0;
while (i < 11) {
    if (i === 5) {
        i = i + 1;
        continue;
    }
    process.stdout.write(String(i));
    if (i < 10) {
        process.stdout.write(", ");
    }
    i = i + 1;
}
console.log("");
