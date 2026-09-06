import sys
# Ported from examples/lumen/exponentiation_naive.lm by scripts/port_examples.py; edit the Lumen original, not this file.
base = 7
exp = 100
mod = 1000000007
iterations = 100
print("Naive exponentiation benchmark")
sys.stdout.write("base = ")
print(base)
sys.stdout.write("exp  = ")
print(exp)
sys.stdout.write("mod  = ")
print(mod)
sys.stdout.write("iterations = ")
print(iterations)
print("")
print("Running naive exponentiation...")
result = 0
i = 0
while i < iterations:
    result = 1
    j = 0
    while j < exp:
        result = result * base
        j = j + 1
    result = result % mod
    i = i + 1
sys.stdout.write("Result: ")
print(result)
print("Done!")
