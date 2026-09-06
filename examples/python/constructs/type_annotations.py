# Ported from examples/lumen/constructs/type_annotations.lm by scripts/port_examples.py; edit the Lumen original, not this file.
print("Test: Type Annotations on Variables")
x = 42
print("let x: number = 42")
print(x)
message = "Hello, World"
print("let message: string = ")
print(message)
flag = True
print("let flag: boolean = true")
print(flag)
empty = None
print("let empty: null = null")
print(empty)
def add(a, b):
    return a + b

print("add(5, 3):")
print(add(5, 3))
def greet(name):
    return "Hello, " + name

print("greet(\"Alice\"):")
print(greet("Alice"))
def process(x, y):
    result = x * 2 + y
    return result

print("process(10, 5):")
print(process(10, 5))
