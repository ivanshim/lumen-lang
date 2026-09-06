# Ported from examples/lumen/constructs/none_type.lm by scripts/port_examples.py; edit the Lumen original, not this file.
def no_return()
    puts("This function returns null implicitly")
end

def explicit_null()
    puts("Returning null explicitly")
    return nil
end

def conditional_null(x)
    if x < 0 then
        return nil
    else
        return x * 2
    end
end

puts("Test: null Type")
puts("Calling no_return():")
result1 = no_return()
puts(result1)
puts("Calling explicit_null():")
result2 = explicit_null()
puts(result2)
puts("conditional_null(5):")
puts(conditional_null(5))
puts("conditional_null(-3):")
puts(conditional_null(-3))
x = nil
puts("let x = null:")
puts(x)
def check_value(val)
    if val == nil then
        puts("Value is null")
    else
        puts("Value is not null")
    end
end

check_value(nil)
check_value(42)
check_value("hello")
