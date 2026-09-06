// Ported from examples/lumen/constructs/operators_complete.lm by scripts/port_examples.py; edit the Lumen original, not this file.
console.log("Test: Complete Operators");
console.log("Logical OR:");
console.log(true || false);
console.log(false || true);
console.log(false || false);
console.log(true || true);
const x = 5;
if (x < 3 || x > 4) {
    console.log("x is either less than 3 or greater than 4");
} else {
    console.log("x is between 3 and 4");
}
console.log("Greater-than-or-equal:");
console.log(10 >= 5);
console.log(5 >= 5);
console.log(3 >= 5);
if (x >= 5) {
    console.log("x is greater than or equal to 5");
} else {
    console.log("x is less than 5");
}
console.log("Exponentiation:");
console.log(2 ** 3);
console.log(5 ** 2);
console.log(10 ** 0);
const result = 2 ** 3 + 1;
console.log("2 ** 3 + 1 = ");
console.log(result);
function power(base, exp) {
    return base ** exp;
}

console.log("power(3, 4):");
console.log(power(3, 4));
console.log("Precedence test: 2 + 3 ** 2");
console.log(2 + 3 ** 2);
console.log("Precedence test: 10 / 2 ** 2");
console.log(10 / 2 ** 2);
