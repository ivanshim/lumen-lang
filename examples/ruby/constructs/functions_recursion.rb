# Ported from examples/lumen/constructs/functions_recursion.lm by scripts/port_examples.py; edit the Lumen original, not this file.
def factorial(n)
    if n <= 1 then
        return 1
    else
        return n * factorial(n - 1)
    end
end

def countdown(n)
    if n <= 0 then
        puts("Done")
    else
        puts(n)
        return countdown(n - 1)
    end
end

puts("Test: Recursion")
puts("Factorial of 5:")
puts(factorial(5))
puts("Countdown from 3:")
countdown(3)
