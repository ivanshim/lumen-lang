# Ported from examples/lumen/constructs/scope_shadowing.lm by scripts/port_examples.py; edit the Lumen original, not this file.
print(1)
x = 10
print(x)
if True:
    x = 20
    print(x)
print(x)
