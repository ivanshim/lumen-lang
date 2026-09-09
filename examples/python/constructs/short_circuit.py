# Ported from examples/lumen/constructs/short_circuit.lm by scripts/port_examples.py; edit the Lumen original, not this file.
def is_even(x):
    print("Checking if ")
    print(x)
    print(" is even")
    return x % 2 == 0

def is_positive(x):
    print("Checking if ")
    print(x)
    print(" is positive")
    return x > 0

print("Test: Short-Circuit Evaluation")
print("false and is_even(10):")
result = False and is_even(10)
print(result)
print("true and is_even(10):")
result = True and is_even(10)
print(result)
print("true or is_positive(5):")
result = True or is_positive(5)
print(result)
print("false or is_positive(5):")
result = False or is_positive(5)
print(result)
print("Testing division by zero avoidance:")
x = 0
if x != 0 and 10 / x > 5:
    print("Result is greater than 5")
else:
    print("x is zero or result is not greater than 5")
def safe_check(value):
    if value != None and value > 10:
        print("Value is not null and greater than 10")
    else:
        print("Value is null or not greater than 10")

safe_check(15)
safe_check(5)
