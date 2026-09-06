// Ported from examples/lumen/constructs/scope_leak.lm by scripts/port_examples.py; edit the Lumen original, not this file.
var y = 100
print(y)
if true {
    y = 50
    print(y)
}
print(y)
