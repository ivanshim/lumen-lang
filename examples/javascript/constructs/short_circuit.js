// Ported from examples/lumen/constructs/short_circuit.lm by scripts/port_examples.py; edit the Lumen original, not this file.
function is_even(x) {
    console.log("Checking if ");
    console.log(x);
    console.log(" is even");
    return x % 2 === 0;
}

function is_positive(x) {
    console.log("Checking if ");
    console.log(x);
    console.log(" is positive");
    return x > 0;
}

console.log("Test: Short-Circuit Evaluation");
console.log("false and is_even(10):");
let result = false && is_even(10);
console.log(result);
console.log("true and is_even(10):");
result = true && is_even(10);
console.log(result);
console.log("true or is_positive(5):");
result = true || is_positive(5);
console.log(result);
console.log("false or is_positive(5):");
result = false || is_positive(5);
console.log(result);
console.log("Testing division by zero avoidance:");
const x = 0;
if (x !== 0 && 10 / x > 5) {
    console.log("Result is greater than 5");
} else {
    console.log("x is zero or result is not greater than 5");
}
function safe_check(value) {
    if (value !== null && value > 10) {
        console.log("Value is not null and greater than 10");
    } else {
        console.log("Value is null or not greater than 10");
    }
}

safe_check(15);
safe_check(5);
