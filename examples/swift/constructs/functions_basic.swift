// Ported from examples/lumen/constructs/functions_basic.lm by scripts/port_examples.py; edit the Lumen original, not this file.
func square(x: Int) -> Int {
    return x * x
}

func add(a: Int, b: Int) -> Int {
    return a + b
}

func greet(name: String) -> String {
    return "Hello, " + name
}

print("Test: Basic Functions")
print(square(x: 5))
print(add(a: 10, b: 20))
print(greet(name: "Lumen"))
func get_constant() -> Int {
    return 42
}

print(get_constant())
func compute(x: Int, y: Int) -> Int {
    let sum = x + y
    let product = x * y
    return sum + product
}

print(compute(x: 3, y: 4))
