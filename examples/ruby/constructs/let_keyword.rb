# Ported from examples/lumen/constructs/let_keyword.lm by scripts/port_examples.py; edit the Lumen original, not this file.
puts("Test: let and let mut Keywords")
x = 10
puts("let x = 10")
puts(x)
y = 5
puts("let mut y = 5")
puts(y)
y = 20
puts("After y = 20:")
puts(y)
x = 100
puts("After let x = 100 (shadowing):")
puts(x)
result = x + y
puts("let result = x + y")
puts(result)
def test_let()
    a = 42
    b = 10
    b = 50
    return a + b
end

puts("test_let():")
puts(test_let())
