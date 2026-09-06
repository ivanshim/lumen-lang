// Ported from examples/lumen/constructs/write_function.lm by scripts/port_examples.py; edit the Lumen original, not this file.
process.stdout.write("Hello");
process.stdout.write(" ");
process.stdout.write("World");
process.stdout.write("!");
console.log("");
process.stdout.write("Numbers: ");
let i = 1;
while (i <= 5) {
    process.stdout.write(String(i));
    if (i < 5) {
        process.stdout.write(", ");
    }
    i = i + 1;
}
console.log("");
