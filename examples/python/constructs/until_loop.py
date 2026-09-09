import sys
# Ported from examples/lumen/constructs/until_loop.lm by scripts/port_examples.py; edit the Lumen original, not this file.
sys.stdout.write("Until loop ascending (0-9): ")
i = 0
while not i >= 10:
    sys.stdout.write(str(i))
    if i < 9:
        sys.stdout.write(", ")
    i = i + 1
print("")
sys.stdout.write("Until loop descending (15-6): ")
x = 15
while not x <= 5:
    sys.stdout.write(str(x))
    if x > 6:
        sys.stdout.write(", ")
    x = x - 1
print("")
