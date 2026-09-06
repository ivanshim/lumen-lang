# Ported from examples/lumen/constructs/comments.lm by scripts/port_examples.py; edit the Lumen original, not this file.
puts("Test: Comments Support")
x = 42
puts(x)
result = x * 2
puts(result)
def add_numbers(a, b)
    return a + b
end

value = add_numbers(10, 20)
puts(value)
if value > 20 then
    puts("Value is greater than 20")
else
    puts("Value is 20 or less")
end
counter = 0
while counter < 3 do
    puts(counter)
    counter = counter + 1
end
puts("Done testing comments")
