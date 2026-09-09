# Ported from examples/lumen/constructs/scope_nested.lm by scripts/port_examples.py; edit the Lumen original, not this file.
x = 1
if True:
    x = 2
    if True:
        x = 3
        print(x)
    print(x)
print(x)
