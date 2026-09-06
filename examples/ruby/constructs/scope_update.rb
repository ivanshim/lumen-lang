# Ported from examples/lumen/constructs/scope_update.lm by scripts/port_examples.py; edit the Lumen original, not this file.
counter = 0
if true then
    counter = counter + 1
end
if true then
    counter = counter + 1
end
puts(counter)
