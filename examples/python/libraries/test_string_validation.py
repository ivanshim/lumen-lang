# Ported from examples/lumen/libraries/test_string_validation.lm by scripts/port_examples.py; edit the Lumen original, not this file.
def is_alpha(c):
    o = ord(c)
    return (o >= ord("A") and o <= ord("Z")) or (o >= ord("a") and o <= ord("z"))

def is_alpha_string(s):
    if len(s) == 0:
        return False
    i = 0
    while i < len(s):
        if not is_alpha(s[i]):
            return False
        i = i + 1
    return True

print("=== String Content Validation ===")
print("")
print("Alphabetic string validation:")
print("  is_alpha_string('hello'): " + str(is_alpha_string("hello")))
print("  is_alpha_string('WORLD'): " + str(is_alpha_string("WORLD")))
print("  is_alpha_string('LuMeN'): " + str(is_alpha_string("LuMeN")))
print("  is_alpha_string('hello123'): " + str(is_alpha_string("hello123")))
print("  is_alpha_string('hello world'): " + str(is_alpha_string("hello world")))
print("  is_alpha_string(''): " + str(is_alpha_string("")))
print("")
print("=== Practical Example: Name Validation ===")
def validate_name_input(s):
    if len(s) == 0:
        print("  '" + s + "' - INVALID: name cannot be empty")
        return False
    if not is_alpha_string(s):
        print("  '" + s + "' - INVALID: name must contain only letters")
        return False
    print("  '" + s + "' - VALID name")
    return True

name_inputs = ["Alice", "Bob123", "Charlie", "", "Dave_Smith"]
i = 0
while i < len(name_inputs):
    validate_name_input(name_inputs[i])
    i = i + 1
