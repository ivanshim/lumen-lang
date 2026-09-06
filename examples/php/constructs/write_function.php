<?php
// Ported from examples/lumen/constructs/write_function.lm by scripts/port_examples.py; edit the Lumen original, not this file.
print("Hello");
print(" ");
print("World");
print("!");
print("\n");
print("Numbers: ");
$i = 1;
while ($i <= 5) {
    print($i);
    if ($i < 5) {
        print(", ");
    }
    $i = $i + 1;
}
print("\n");
