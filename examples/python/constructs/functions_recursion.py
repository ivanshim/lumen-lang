# Ported from examples/lumen/constructs/functions_recursion.lm by scripts/port_examples.py; edit the Lumen original, not this file.
def factorial(n):
    if n <= 1:
        return 1
    else:
        return n * factorial(n - 1)

def countdown(n):
    if n <= 0:
        print("Done")
    else:
        print(n)
        return countdown(n - 1)

print("Test: Recursion")
print("Factorial of 5:")
print(factorial(5))
print("Countdown from 3:")
countdown(3)
