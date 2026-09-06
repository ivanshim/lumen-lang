import sys
# Ported from examples/lumen/constructs/round_function.lm by scripts/port_examples.py; edit the Lumen original, not this file.
def round(x, decimals):
    scale = 1
    i = 0
    while i < decimals:
        scale = scale * 10
        i = i + 1
    y = x * scale
    if y >= 0:
        r = (y * 2 + 1) // 2
    else:
        r = (y * 2 - 1) // 2
    return r / scale

sys.stdout.write("Positive number tests:")
sys.stdout.write("round(1.235, 2) = ")
print(round(1.235, 2))
sys.stdout.write("round(1.234, 2) = ")
print(round(1.234, 2))
sys.stdout.write("round(1.245, 2) = ")
print(round(1.245, 2))
sys.stdout.write("round(1.5, 0) = ")
print(round(1.5, 0))
sys.stdout.write("round(2.5, 0) = ")
print(round(2.5, 0))
sys.stdout.write("\nNegative number tests (round half away from zero):")
sys.stdout.write("round(-1.235, 2) = ")
print(round(-1.235, 2))
sys.stdout.write("round(-1.234, 2) = ")
print(round(-1.234, 2))
sys.stdout.write("round(-1.245, 2) = ")
print(round(-1.245, 2))
sys.stdout.write("round(-1.5, 0) = ")
print(round(-1.5, 0))
sys.stdout.write("round(-2.5, 0) = ")
print(round(-2.5, 0))
sys.stdout.write("\nBoundary cases:")
sys.stdout.write("round(0, 2) = ")
print(round(0, 2))
sys.stdout.write("round(0.005, 2) = ")
print(round(0.005, 2))
sys.stdout.write("round(0.004, 2) = ")
print(round(0.004, 2))
sys.stdout.write("round(10.999, 2) = ")
print(round(10.999, 2))
sys.stdout.write("round(99.995, 2) = ")
print(round(99.995, 2))
sys.stdout.write("\nMultiple decimal places with pi:")
sys.stdout.write("round(3.14159, 2) = ")
print(round(3.14159, 2))
sys.stdout.write("round(3.14159, 3) = ")
print(round(3.14159, 3))
sys.stdout.write("round(3.14159, 4) = ")
print(round(3.14159, 4))
