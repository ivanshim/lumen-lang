// Ported from examples/lumen/constructs/scope_shadowing.lm by scripts/port_examples.py; edit the Lumen original, not this file.
print(1)
var x = 10
print(x)
if true {
    x = 20
    print(x)
}
print(x)
