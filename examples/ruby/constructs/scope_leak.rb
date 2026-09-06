# Ported from examples/lumen/constructs/scope_leak.lm by scripts/port_examples.py; edit the Lumen original, not this file.
y = 100
puts(y)
if true then
    y = 50
    puts(y)
end
puts(y)
