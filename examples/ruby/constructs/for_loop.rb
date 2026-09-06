# Ported from examples/lumen/constructs/for_loop.lm by scripts/port_examples.py; edit the Lumen original, not this file.
for i in 0...10 do
    print(i)
    if i < 9 then
        print(", ")
    end
end
puts("")
