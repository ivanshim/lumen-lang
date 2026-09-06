// Ported from examples/lumen/constructs/demo.lm by scripts/port_examples.py; edit the Lumen original, not this file.
console.log(1 + 2 * 3);
const x = 0;
const y = 5;
if (x < y && y === 5) {
    console.log(100);
} else {
    console.log(200);
}
let i = 0;
let sum = 0;
while (i < 10) {
    if (i === 5) {
        i = i + 1;
        continue;
    }
    if (i === 8) {
        break;
    }
    sum = sum + i;
    console.log(sum);
    i = i + 1;
}
console.log(sum);
console.log(true);
console.log(false);
console.log(!false);
console.log(-10 + 3);
