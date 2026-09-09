# Ported from examples/lumen/test_scope_leak_fix.lm by scripts/port_examples.py; edit the Lumen original, not this file.
def count_to_three():
    i = 0
    while i < 3:
        i = i + 1
    return i

print("First call: " + str(count_to_three()))
print("Second call: " + str(count_to_three()))
print("Third call: " + str(count_to_three()))
print("All calls completed successfully!")
