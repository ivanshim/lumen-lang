# Ported from examples/lumen/constructs/functions_basic.lm by scripts/port_examples.py; edit the Lumen original, not this file.
def square(x):
    return x * x

def add(a, b):
    return a + b

def greet(name):
    return "Hello, " + name

print("Test: Basic Functions")
print(square(5))
print(add(10, 20))
print(greet("Lumen"))
def get_constant():
    return 42

print(get_constant())
def compute(x, y):
    sum = x + y
    product = x * y
    return sum + product

print(compute(3, 4))
