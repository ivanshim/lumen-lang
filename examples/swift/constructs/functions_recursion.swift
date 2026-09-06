// Ported from examples/lumen/constructs/functions_recursion.lm by scripts/port_examples.py; edit the Lumen original, not this file.
func factorial(n: Int) -> Int {
    if n <= 1 {
        return 1
    } else {
        return n * factorial(n: n - 1)
    }
}

func countdown(n: Int) -> Int {
    if n <= 0 {
        print("Done")
    } else {
        print(n)
        return countdown(n: n - 1)
    }
}

print("Test: Recursion")
print("Factorial of 5:")
print(factorial(n: 5))
print("Countdown from 3:")
countdown(n: 3)
