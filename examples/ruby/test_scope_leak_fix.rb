# Ported from examples/lumen/test_scope_leak_fix.lm by scripts/port_examples.py; edit the Lumen original, not this file.
def count_to_three()
    i = 0
    while i < 3 do
        i = i + 1
    end
    return i
end

puts("First call: " + count_to_three().to_s)
puts("Second call: " + count_to_three().to_s)
puts("Third call: " + count_to_three().to_s)
puts("All calls completed successfully!")
