# Ported from examples/lumen/libraries/test_base_minimal.lm by scripts/port_examples.py; edit the Lumen original, not this file.
alphabet = "0123456789abcdefghijklmnopqrstuvwxyz"
def integer_to_base_string(n, radix):
    if n == 0:
        return "0"
    negative = False
    if n < 0:
        negative = True
        n = -n
    result = ""
    while n > 0:
        digit = n % radix
        result = alphabet[digit] + result
        n = n // radix
    if negative:
        result = "-" + result
    return result

print("Test: Convert 255 to hex")
result = integer_to_base_string(255, 16)
print(result)
print("Test: Convert 42 to binary")
result2 = integer_to_base_string(42, 2)
print(result2)
print("Done!")
