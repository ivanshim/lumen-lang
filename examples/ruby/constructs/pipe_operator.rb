# Ported from examples/lumen/constructs/pipe_operator.lm by scripts/port_examples.py; edit the Lumen original, not this file.
def double(x)
    return x * 2
end

def add_one(x)
    return x + 1
end

def square(x)
    return x * x
end

puts("Test: Pipe Operator")
puts("Without pipe: square(add_one(double(5)))")
puts(square(add_one(double(5))))
puts("With pipe: 5 |> double() |> add_one() |> square()")
result = square(add_one(double(5)))
puts(result)
puts("10 |> double():")
puts(double(10))
def multiply(a, b)
    return a * b
end

puts("3 |> double():")
x = double(3)
puts(multiply(x, 2))
