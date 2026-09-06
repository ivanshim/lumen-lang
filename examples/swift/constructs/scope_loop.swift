// Ported from examples/lumen/constructs/scope_loop.lm by scripts/port_examples.py; edit the Lumen original, not this file.
var i = 0
var sum = 0
while i < 5 {
    sum = sum + i
    print(sum)
    i = i + 1
}
print(i)
print(sum)
