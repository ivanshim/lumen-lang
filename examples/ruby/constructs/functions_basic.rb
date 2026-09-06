# Ported from examples/lumen/constructs/functions_basic.lm by scripts/port_examples.py; edit the Lumen original, not this file.
def square(x)
    return x * x
end

def add(a, b)
    return a + b
end

def greet(name)
    return "Hello, " + name
end

puts("Test: Basic Functions")
puts(square(5))
puts(add(10, 20))
puts(greet("Lumen"))
def get_constant()
    return 42
end

puts(get_constant())
def compute(x, y)
    sum = x + y
    product = x * y
    return sum + product
end

puts(compute(3, 4))
