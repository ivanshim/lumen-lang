# Ported from examples/lumen/constructs/for_loop_control.lm by scripts/port_examples.py; edit the Lumen original, not this file.
for i in 0...15 do
    if i == 10 then
        break
    end
    print(i)
    if i < 9 then
        print(", ")
    end
end
puts("")
