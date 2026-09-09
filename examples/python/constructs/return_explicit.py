# Ported from examples/lumen/constructs/return_explicit.lm by scripts/port_examples.py; edit the Lumen original, not this file.
def absolute(x):
    if x < 0:
        return -x
    return x

def safe_divide(a, b):
    if b == 0:
        return None
    return a / b

def find_first_even(a, b, c):
    if a % 2 == 0:
        return a
    if b % 2 == 0:
        return b
    return c

print("Test: Explicit Returns")
print("absolute(-5):")
print(absolute(-5))
print(absolute(5))
print("safe_divide(10, 2):")
print(safe_divide(10, 2))
print("safe_divide(10, 0):")
print(safe_divide(10, 0))
print("find_first_even(1, 2, 3):")
print(find_first_even(1, 2, 3))
print("find_first_even(2, 5, 7):")
print(find_first_even(2, 5, 7))
