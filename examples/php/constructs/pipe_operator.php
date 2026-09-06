<?php
// Ported from examples/lumen/constructs/pipe_operator.lm by scripts/port_examples.py; edit the Lumen original, not this file.
function double($x) {
    return $x * 2;
}

function add_one($x) {
    return $x + 1;
}

function square($x) {
    return $x * $x;
}

print("Test: Pipe Operator\n");
print("Without pipe: square(add_one(double(5)))\n");
print(square(add_one(double(5))) . "\n");
print("With pipe: 5 |> double() |> add_one() |> square()\n");
$result = square(add_one(double(5)));
print($result . "\n");
print("10 |> double():\n");
print(double(10) . "\n");
function multiply($a, $b) {
    return $a * $b;
}

print("3 |> double():\n");
$x = double(3);
print(multiply($x, 2) . "\n");
