// Ported from examples/lumen/constructs/comments.lm by scripts/port_examples.py; edit the Lumen original, not this file.
console.log("Test: Comments Support");
const x = 42;
console.log(x);
const result = x * 2;
console.log(result);
function add_numbers(a, b) {
    return a + b;
}

const value = add_numbers(10, 20);
console.log(value);
if (value > 20) {
    console.log("Value is greater than 20");
} else {
    console.log("Value is 20 or less");
}
let counter = 0;
while (counter < 3) {
    console.log(counter);
    counter = counter + 1;
}
console.log("Done testing comments");
