import sys
# Ported from examples/lumen/exponentiation_exponent_then_mod.lm by scripts/port_examples.py; edit the Lumen original, not this file.
def mod_pow(base, exp, m):
    result = 1
    base = base % m
    while exp > 0:
        if exp % 2 == 1:
            result = (result * base) % m
        exp = exp // 2
        base = (base * base) % m
    return result

base = 7
exp = 100
mod = 1000000007
iterations = 100
print("Fast modular exponentiation benchmark")
sys.stdout.write("base = ")
print(base)
sys.stdout.write("exp  = ")
print(exp)
sys.stdout.write("mod  = ")
print(mod)
sys.stdout.write("iterations = ")
print(iterations)
print("")
print("Running mod_pow...")
result = 0
i = 0
while i < iterations:
    result = mod_pow(base, exp, mod)
    i = i + 1
sys.stdout.write("Result: ")
print(result)
print("Done!")
