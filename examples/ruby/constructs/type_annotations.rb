# Ported from examples/lumen/constructs/type_annotations.lm by scripts/port_examples.py; edit the Lumen original, not this file.
puts("Test: Type Annotations on Variables")
x = 42
puts("let x: number = 42")
puts(x)
message = "Hello, World"
puts("let message: string = ")
puts(message)
flag = true
puts("let flag: boolean = true")
puts(flag)
empty = nil
puts("let empty: null = null")
puts(empty)
def add(a, b)
    return a + b
end

puts("add(5, 3):")
puts(add(5, 3))
def greet(name)
    return "Hello, " + name
end

puts("greet(\"Alice\"):")
puts(greet("Alice"))
def process(x, y)
    result = x * 2 + y
    return result
end

puts("process(10, 5):")
puts(process(10, 5))
