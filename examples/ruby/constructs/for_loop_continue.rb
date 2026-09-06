# Ported from examples/lumen/constructs/for_loop_continue.lm by scripts/port_examples.py; edit the Lumen original, not this file.
for i in 0...11 do
    if i == 5 then
        next
    end
    print(i)
    if i < 10 then
        print(", ")
    end
end
puts("")
