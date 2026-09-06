# Ported from examples/lumen/constructs/scope_if.lm by scripts/port_examples.py; edit the Lumen original, not this file.
x = 10
if true then
    x = 20
    puts(x)
else
    x = 30
    puts(x)
end
puts(x)
