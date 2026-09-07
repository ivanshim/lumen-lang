# Ported from examples/lumen/exponentiation_exponent_then_mod.lm by scripts/port_examples.py; edit the Lumen original, not this file.
base = 7
exp = 100
mod = 1000000007
iterations = 100
puts("Fast modular exponentiation benchmark")
print("base = ")
puts(base)
print("exp  = ")
puts(exp)
print("mod  = ")
puts(mod)
print("iterations = ")
puts(iterations)
puts("")
puts("Running mod_pow...")
result = 0
i = 0
while i < iterations do
    result = mod_pow(base, exp, mod)
    i = i + 1
end
print("Result: ")
puts(result)
puts("Done!")
