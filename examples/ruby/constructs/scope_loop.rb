# Ported from examples/lumen/constructs/scope_loop.lm by scripts/port_examples.py; edit the Lumen original, not this file.
i = 0
sum = 0
while i < 5 do
    sum = sum + i
    puts(sum)
    i = i + 1
end
puts(i)
puts(sum)
