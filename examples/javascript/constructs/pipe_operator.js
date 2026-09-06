// Ported from examples/lumen/constructs/pipe_operator.lm by scripts/port_examples.py; edit the Lumen original, not this file.
function double(x) {
    return x * 2;
}

function add_one(x) {
    return x + 1;
}

function square(x) {
    return x * x;
}

console.log("Test: Pipe Operator");
console.log("Without pipe: square(add_one(double(5)))");
console.log(square(add_one(double(5))));
console.log("With pipe: 5 |> double() |> add_one() |> square()");
const result = square(add_one(double(5)));
console.log(result);
console.log("10 |> double():");
console.log(double(10));
function multiply(a, b) {
    return a * b;
}

console.log("3 |> double():");
const x = double(3);
console.log(multiply(x, 2));
