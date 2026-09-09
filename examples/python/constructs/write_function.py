import sys
# Ported from examples/lumen/constructs/write_function.lm by scripts/port_examples.py; edit the Lumen original, not this file.
sys.stdout.write("Hello")
sys.stdout.write(" ")
sys.stdout.write("World")
sys.stdout.write("!")
print("")
sys.stdout.write("Numbers: ")
i = 1
while i <= 5:
    sys.stdout.write(str(i))
    if i < 5:
        sys.stdout.write(", ")
    i = i + 1
print("")
