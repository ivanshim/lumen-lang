# Ported from examples/lumen/constructs/until_loop.lm by scripts/port_examples.py; edit the Lumen original, not this file.
print("Until loop ascending (0-9): ")
i = 0
while !(i >= 10) do
    print(i)
    if i < 9 then
        print(", ")
    end
    i = i + 1
end
puts("")
print("Until loop descending (15-6): ")
x = 15
while !(x <= 5) do
    print(x)
    if x > 6 then
        print(", ")
    end
    x = x - 1
end
puts("")
