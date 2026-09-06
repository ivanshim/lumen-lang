# Ported from examples/lumen/libraries/test_string_validation.lm by scripts/port_examples.py; edit the Lumen original, not this file.
def is_alpha(c)
    o = c.ord
    return (o >= "A".ord && o <= "Z".ord) || (o >= "a".ord && o <= "z".ord)
end

def is_alpha_string(s)
    if s.length == 0 then
        return false
    end
    i = 0
    while i < s.length do
        if !is_alpha(s[i]) then
            return false
        end
        i = i + 1
    end
    return true
end

puts("=== String Content Validation ===")
puts("")
puts("Alphabetic string validation:")
puts("  is_alpha_string('hello'): " + is_alpha_string("hello").to_s)
puts("  is_alpha_string('WORLD'): " + is_alpha_string("WORLD").to_s)
puts("  is_alpha_string('LuMeN'): " + is_alpha_string("LuMeN").to_s)
puts("  is_alpha_string('hello123'): " + is_alpha_string("hello123").to_s)
puts("  is_alpha_string('hello world'): " + is_alpha_string("hello world").to_s)
puts("  is_alpha_string(''): " + is_alpha_string("").to_s)
puts("")
puts("=== Practical Example: Name Validation ===")
def validate_name_input(s)
    if s.length == 0 then
        puts("  '" + s + "' - INVALID: name cannot be empty")
        return false
    end
    if !is_alpha_string(s) then
        puts("  '" + s + "' - INVALID: name must contain only letters")
        return false
    end
    puts("  '" + s + "' - VALID name")
    return true
end

name_inputs = ["Alice", "Bob123", "Charlie", "", "Dave_Smith"]
i = 0
while i < name_inputs.length do
    validate_name_input(name_inputs[i])
    i = i + 1
end
