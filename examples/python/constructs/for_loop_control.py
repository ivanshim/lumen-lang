import sys
# Ported from examples/lumen/constructs/for_loop_control.lm by scripts/port_examples.py; edit the Lumen original, not this file.
for i in range(0, 15):
    if i == 10:
        break
    sys.stdout.write(str(i))
    if i < 9:
        sys.stdout.write(", ")
print("")
