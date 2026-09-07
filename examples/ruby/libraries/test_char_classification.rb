# Ported from examples/lumen/libraries/test_char_classification.lm by scripts/port_examples.py; edit the Lumen original, not this file.
puts("=== Character Classification ===")
puts("")
puts("ASCII characters:")
puts("  is_ascii('A'): " + is_ascii("A").to_s)
puts("  is_ascii('5'): " + is_ascii("5").to_s)
puts("  is_ascii(' '): " + is_ascii(" ").to_s)
puts("")
puts("Digit detection:")
puts("  is_digit('0'): " + is_digit("0").to_s)
puts("  is_digit('9'): " + is_digit("9").to_s)
puts("  is_digit('a'): " + is_digit("a").to_s)
puts("  is_digit('A'): " + is_digit("A").to_s)
puts("")
puts("Alphabetic detection:")
puts("  is_alpha('a'): " + is_alpha("a").to_s)
puts("  is_alpha('Z'): " + is_alpha("Z").to_s)
puts("  is_alpha('5'): " + is_alpha("5").to_s)
puts("  is_alpha('!'): " + is_alpha("!").to_s)
puts("")
puts("Alphanumeric detection:")
puts("  is_alnum('a'): " + is_alnum("a").to_s)
puts("  is_alnum('Z'): " + is_alnum("Z").to_s)
puts("  is_alnum('5'): " + is_alnum("5").to_s)
puts("  is_alnum('!'): " + is_alnum("!").to_s)
puts("  is_alnum(' '): " + is_alnum(" ").to_s)
puts("")
puts("=== Practical Example: Username Validation ===")
def is_valid_username_char(c)
    return is_alnum(c) || c == "_" || c == "-"
end

test_chars = ["a", "Z", "5", "_", "-", "!", "@"]
i = 0
while i < test_chars.length do
    c = test_chars[i]
    valid = is_valid_username_char(c)
    puts("  '" + c + "' is valid username char: " + valid.to_s)
    i = i + 1
end
