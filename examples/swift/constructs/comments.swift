// Ported from examples/lumen/constructs/comments.lm by scripts/port_examples.py; edit the Lumen original, not this file.
print("Test: Comments Support")
let x = 42
print(x)
let result = x * 2
print(result)
func add_numbers(a: Int, b: Int) -> Int {
    return a + b
}

let value = add_numbers(a: 10, b: 20)
print(value)
if value > 20 {
    print("Value is greater than 20")
} else {
    print("Value is 20 or less")
}
var counter = 0
while counter < 3 {
    print(counter)
    counter = counter + 1
}
print("Done testing comments")
