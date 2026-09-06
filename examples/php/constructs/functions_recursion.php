<?php
// Ported from examples/lumen/constructs/functions_recursion.lm by scripts/port_examples.py; edit the Lumen original, not this file.
function factorial($n) {
    if ($n <= 1) {
        return 1;
    } else {
        return $n * factorial($n - 1);
    }
}

function countdown($n) {
    if ($n <= 0) {
        print("Done\n");
    } else {
        print($n . "\n");
        return countdown($n - 1);
    }
}

print("Test: Recursion\n");
print("Factorial of 5:\n");
print(factorial(5) . "\n");
print("Countdown from 3:\n");
countdown(3);
