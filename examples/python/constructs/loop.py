import sys
# Ported from examples/lumen/constructs/loop.lm by scripts/port_examples.py; edit the Lumen original, not this file.
x = 0
while x < 10:
    sys.stdout.write(str(x))
    if x < 9:
        sys.stdout.write(", ")
    x = x + 1
print("")
