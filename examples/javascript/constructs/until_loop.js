// Ported from examples/lumen/constructs/until_loop.lm by scripts/port_examples.py; edit the Lumen original, not this file.
process.stdout.write("Until loop ascending (0-9): ");
let i = 0;
while (!(i >= 10)) {
    process.stdout.write(String(i));
    if (i < 9) {
        process.stdout.write(", ");
    }
    i = i + 1;
}
console.log("");
process.stdout.write("Until loop descending (15-6): ");
let x = 15;
while (!(x <= 5)) {
    process.stdout.write(String(x));
    if (x > 6) {
        process.stdout.write(", ");
    }
    x = x - 1;
}
console.log("");
