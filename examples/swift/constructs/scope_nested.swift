// Ported from examples/lumen/constructs/scope_nested.lm by scripts/port_examples.py; edit the Lumen original, not this file.
var x = 1
if true {
    x = 2
    if true {
        x = 3
        print(x)
    }
    print(x)
}
print(x)
