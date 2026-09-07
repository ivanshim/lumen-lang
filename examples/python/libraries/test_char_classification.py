# Ported from examples/lumen/libraries/test_char_classification.lm by scripts/port_examples.py; edit the Lumen original, not this file.
print("=== Character Classification ===")
print("")
print("ASCII characters:")
print("  is_ascii('A'): " + str(is_ascii("A")))
print("  is_ascii('5'): " + str(is_ascii("5")))
print("  is_ascii(' '): " + str(is_ascii(" ")))
print("")
print("Digit detection:")
print("  is_digit('0'): " + str(is_digit("0")))
print("  is_digit('9'): " + str(is_digit("9")))
print("  is_digit('a'): " + str(is_digit("a")))
print("  is_digit('A'): " + str(is_digit("A")))
print("")
print("Alphabetic detection:")
print("  is_alpha('a'): " + str(is_alpha("a")))
print("  is_alpha('Z'): " + str(is_alpha("Z")))
print("  is_alpha('5'): " + str(is_alpha("5")))
print("  is_alpha('!'): " + str(is_alpha("!")))
print("")
print("Alphanumeric detection:")
print("  is_alnum('a'): " + str(is_alnum("a")))
print("  is_alnum('Z'): " + str(is_alnum("Z")))
print("  is_alnum('5'): " + str(is_alnum("5")))
print("  is_alnum('!'): " + str(is_alnum("!")))
print("  is_alnum(' '): " + str(is_alnum(" ")))
print("")
print("=== Practical Example: Username Validation ===")
def is_valid_username_char(c):
    return is_alnum(c) or c == "_" or c == "-"

test_chars = ["a", "Z", "5", "_", "-", "!", "@"]
i = 0
while i < len(test_chars):
    c = test_chars[i]
    valid = is_valid_username_char(c)
    print("  '" + c + "' is valid username char: " + str(valid))
    i = i + 1
