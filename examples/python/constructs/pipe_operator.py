# Ported from examples/lumen/constructs/pipe_operator.lm by scripts/port_examples.py; edit the Lumen original, not this file.
def double(x):
    return x * 2

def add_one(x):
    return x + 1

def square(x):
    return x * x

print("Test: Pipe Operator")
print("Without pipe: square(add_one(double(5)))")
print(square(x= add_one(x= double(x= 5))))
print("With pipe: 5 |> double() |> add_one() |> square()")
result = square(x= add_one(x= double(x= 5)))
print(result)
print("10 |> double():")
print(double(x= 10))
def multiply(a, b):
    return a * b

print("3 |> double():")
x = double(x= 3)
print(multiply(a= x, b= 2))
