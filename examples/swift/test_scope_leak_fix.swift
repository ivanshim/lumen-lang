// Ported from examples/lumen/test_scope_leak_fix.lm by scripts/port_examples.py; edit the Lumen original, not this file.
func count_to_three() -> Int {
    var i = 0
    while i < 3 {
        i = i + 1
    }
    return i
}

print("First call: " + String(count_to_three()))
print("Second call: " + String(count_to_three()))
print("Third call: " + String(count_to_three()))
print("All calls completed successfully!")
