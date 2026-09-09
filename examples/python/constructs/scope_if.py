# Ported from examples/lumen/constructs/scope_if.lm by scripts/port_examples.py; edit the Lumen original, not this file.
x = 10
if True:
    x = 20
    print(x)
else:
    x = 30
    print(x)
print(x)
