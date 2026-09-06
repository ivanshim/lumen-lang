# Ported from examples/lumen/constructs/let_keyword.lm by scripts/port_examples.py; edit the Lumen original, not this file.
print("Test: let and let mut Keywords")
x = 10
print("let x = 10")
print(x)
y = 5
print("let mut y = 5")
print(y)
y = 20
print("After y = 20:")
print(y)
x = 100
print("After let x = 100 (shadowing):")
print(x)
result = x + y
print("let result = x + y")
print(result)
def test_let():
    a = 42
    b = 10
    b = 50
    return a + b

print("test_let():")
print(test_let())
