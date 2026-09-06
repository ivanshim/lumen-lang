# Ported from examples/lumen/constructs/write_function.lm by scripts/port_examples.py; edit the Lumen original, not this file.
print("Hello")
print(" ")
print("World")
print("!")
puts("")
print("Numbers: ")
i = 1
while i <= 5 do
    print(i)
    if i < 5 then
        print(", ")
    end
    i = i + 1
end
puts("")
