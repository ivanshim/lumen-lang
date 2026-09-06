# Ported from examples/lumen/constructs/scope_nested.lm by scripts/port_examples.py; edit the Lumen original, not this file.
x = 1
if true then
    x = 2
    if true then
        x = 3
        puts(x)
    end
    puts(x)
end
puts(x)
