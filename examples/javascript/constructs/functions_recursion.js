// Ported from examples/lumen/constructs/functions_recursion.lm by scripts/port_examples.py; edit the Lumen original, not this file.
function factorial(n) {
    if (n <= 1) {
        return 1;
    } else {
        return n * factorial(n - 1);
    }
}

function countdown(n) {
    if (n <= 0) {
        console.log("Done");
    } else {
        console.log(n);
        return countdown(n - 1);
    }
}

console.log("Test: Recursion");
console.log("Factorial of 5:");
console.log(factorial(5));
console.log("Countdown from 3:");
countdown(3);
