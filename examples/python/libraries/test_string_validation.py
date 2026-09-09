# Ported from examples/lumen/libraries/test_string_validation.lm by scripts/port_examples.py; edit the Lumen original, not this file.
print("=== String Content Validation ===")
print("")
print("Alphabetic string validation:")
print("  is_alpha_string('hello'): " + str(is_alpha_string(s= "hello")))
print("  is_alpha_string('WORLD'): " + str(is_alpha_string(s= "WORLD")))
print("  is_alpha_string('LuMeN'): " + str(is_alpha_string(s= "LuMeN")))
print("  is_alpha_string('hello123'): " + str(is_alpha_string(s= "hello123")))
print("  is_alpha_string('hello world'): " + str(is_alpha_string(s= "hello world")))
print("  is_alpha_string(''): " + str(is_alpha_string(s= "")))
print("")
print("=== Practical Example: Name Validation ===")
def validate_name_input(s):
    if len(s) == 0:
        print("  '" + s + "' - INVALID: name cannot be empty")
        return False
    if not is_alpha_string(s= s):
        print("  '" + s + "' - INVALID: name must contain only letters")
        return False
    print("  '" + s + "' - VALID name")
    return True

name_inputs = ["Alice", "Bob123", "Charlie", "", "Dave_Smith"]
i = 0
while i < len(name_inputs):
    validate_name_input(s= name_inputs[i])
    i = i + 1
