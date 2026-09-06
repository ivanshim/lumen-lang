<?php
// Ported from examples/lumen/test_scope_leak_fix.lm by scripts/port_examples.py; edit the Lumen original, not this file.
function count_to_three() {
    $i = 0;
    while ($i < 3) {
        $i = $i + 1;
    }
    return $i;
}

print(("First call: " . strval(count_to_three())) . "\n");
print(("Second call: " . strval(count_to_three())) . "\n");
print(("Third call: " . strval(count_to_three())) . "\n");
print("All calls completed successfully!\n");
