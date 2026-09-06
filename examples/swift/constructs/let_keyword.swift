// Ported from examples/lumen/constructs/let_keyword.lm by scripts/port_examples.py; edit the Lumen original, not this file.
print("Test: let and let mut Keywords")
var x = 10
print("let x = 10")
print(x)
var y = 5
print("let mut y = 5")
print(y)
y = 20
print("After y = 20:")
print(y)
x = 100
print("After let x = 100 (shadowing):")
print(x)
let result = x + y
print("let result = x + y")
print(result)
func test_let() -> Int {
    let a = 42
    var b = 10
    b = 50
    return a + b
}

print("test_let():")
print(test_let())
