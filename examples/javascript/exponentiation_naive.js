// Ported from examples/lumen/exponentiation_naive.lm by scripts/port_examples.py; edit the Lumen original, not this file.
let j;
const base = 7;
const exp = 100;
const mod = 1000000007;
const iterations = 100;
console.log("Naive exponentiation benchmark");
process.stdout.write("base = ");
console.log(base);
process.stdout.write("exp  = ");
console.log(exp);
process.stdout.write("mod  = ");
console.log(mod);
process.stdout.write("iterations = ");
console.log(iterations);
console.log("");
console.log("Running naive exponentiation...");
let result = 0;
let i = 0;
while (i < iterations) {
    result = 1;
    j = 0;
    while (j < exp) {
        result = result * base;
        j = j + 1;
    }
    result = result % mod;
    i = i + 1;
}
process.stdout.write("Result: ");
console.log(result);
console.log("Done!");
