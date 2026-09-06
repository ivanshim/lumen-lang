import sys
# Ported from examples/lumen/constructs/for_loop_continue.lm by scripts/port_examples.py; edit the Lumen original, not this file.
for i in range(0, 11):
    if i == 5:
        continue
    sys.stdout.write(str(i))
    if i < 10:
        sys.stdout.write(", ")
print("")
