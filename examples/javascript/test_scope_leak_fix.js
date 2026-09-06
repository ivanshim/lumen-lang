// Ported from examples/lumen/test_scope_leak_fix.lm by scripts/port_examples.py; edit the Lumen original, not this file.
function count_to_three() {
    let i = 0;
    while (i < 3) {
        i = i + 1;
    }
    return i;
}

console.log("First call: " + String(count_to_three()));
console.log("Second call: " + String(count_to_three()));
console.log("Third call: " + String(count_to_three()));
console.log("All calls completed successfully!");
