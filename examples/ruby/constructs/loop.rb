# Ported from examples/lumen/constructs/loop.lm by scripts/port_examples.py; edit the Lumen original, not this file.
x = 0
while x < 10 do
    print(x)
    if x < 9 then
        print(", ")
    end
    x = x + 1
end
puts("")
