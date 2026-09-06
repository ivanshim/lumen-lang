// Ported from examples/lumen/constructs/pipe_operator.lm by scripts/port_examples.py; edit the Lumen original, not this file.
func double(x: Int) -> Int {
    return x * 2
}

func add_one(x: Int) -> Int {
    return x + 1
}

func square(x: Int) -> Int {
    return x * x
}

print("Test: Pipe Operator")
print("Without pipe: square(add_one(double(5)))")
print(square(x: add_one(x: double(x: 5))))
print("With pipe: 5 |> double() |> add_one() |> square()")
let result = square(x: add_one(x: double(x: 5)))
print(result)
print("10 |> double():")
print(double(x: 10))
func multiply(a: Int, b: Int) -> Int {
    return a * b
}

print("3 |> double():")
let x = double(x: 3)
print(multiply(a: x, b: 2))
