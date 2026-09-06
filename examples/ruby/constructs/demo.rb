# Ported from examples/lumen/constructs/demo.lm by scripts/port_examples.py; edit the Lumen original, not this file.
puts(1 + 2 * 3)
x = 0
y = 5
if x < y && y == 5 then
    puts(100)
else
    puts(200)
end
i = 0
sum = 0
while i < 10 do
    if i == 5 then
        i = i + 1
        next
    end
    if i == 8 then
        break
    end
    sum = sum + i
    puts(sum)
    i = i + 1
end
puts(sum)
puts(true)
puts(false)
puts(!false)
puts(-10 + 3)
