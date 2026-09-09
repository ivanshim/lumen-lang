# Ported from examples/lumen/constructs/comments.lm by scripts/port_examples.py; edit the Lumen original, not this file.
print("Test: Comments Support")
x = 42
print(x)
result = x * 2
print(result)
def add_numbers(a, b):
    return a + b

value = add_numbers(10, 20)
print(value)
if value > 20:
    print("Value is greater than 20")
else:
    print("Value is 20 or less")
counter = 0
while counter < 3:
    print(counter)
    counter = counter + 1
print("Done testing comments")
