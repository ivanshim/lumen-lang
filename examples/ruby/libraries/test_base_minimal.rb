# Ported from examples/lumen/libraries/test_base_minimal.lm by scripts/port_examples.py; edit the Lumen original, not this file.
alphabet = "0123456789abcdefghijklmnopqrstuvwxyz"
def integer_to_base_string(n, radix)
    alphabet = "0123456789abcdefghijklmnopqrstuvwxyz"
    if n == 0 then
        return "0"
    end
    negative = false
    if n < 0 then
        negative = true
        n = -n
    end
    result = ""
    while n > 0 do
        digit = n % radix
        result = alphabet[digit] + result
        n = n / radix
    end
    if negative then
        result = "-" + result
    end
    return result
end

puts("Test: Convert 255 to hex")
result = integer_to_base_string(255, 16)
puts(result)
puts("Test: Convert 42 to binary")
result2 = integer_to_base_string(42, 2)
puts(result2)
puts("Done!")
