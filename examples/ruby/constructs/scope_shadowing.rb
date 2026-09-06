# Ported from examples/lumen/constructs/scope_shadowing.lm by scripts/port_examples.py; edit the Lumen original, not this file.
puts(1)
x = 10
puts(x)
if true then
    x = 20
    puts(x)
end
puts(x)
