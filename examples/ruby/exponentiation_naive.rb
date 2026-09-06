# Ported from examples/lumen/exponentiation_naive.lm by scripts/port_examples.py; edit the Lumen original, not this file.
base = 7
exp = 100
mod = 1000000007
iterations = 100
puts("Naive exponentiation benchmark")
print("base = ")
puts(base)
print("exp  = ")
puts(exp)
print("mod  = ")
puts(mod)
print("iterations = ")
puts(iterations)
puts("")
puts("Running naive exponentiation...")
result = 0
i = 0
while i < iterations do
    result = 1
    j = 0
    while j < exp do
        result = result * base
        j = j + 1
    end
    result = result % mod
    i = i + 1
end
print("Result: ")
puts(result)
puts("Done!")
