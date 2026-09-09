# Ported from examples/lumen/libraries/test_string_transform.lm by scripts/port_examples.py; edit the Lumen original, not this file.
print("=== String Case Transformation ===")
print("")
print("Uppercase conversion:")
print("  string_to_upper('hello'): " + string_to_upper(s= "hello"))
print("  string_to_upper('world'): " + string_to_upper(s= "world"))
print("  string_to_upper('Hello123'): " + string_to_upper(s= "Hello123"))
print("")
print("Lowercase conversion:")
print("  string_to_lower('HELLO'): " + string_to_lower(s= "HELLO"))
print("  string_to_lower('WORLD'): " + string_to_lower(s= "WORLD"))
print("  string_to_lower('Hello123'): " + string_to_lower(s= "Hello123"))
print("")
print("Single character transformations:")
print("  char_to_upper('a'): " + char_to_upper(c= "a"))
print("  char_to_upper('z'): " + char_to_upper(c= "z"))
print("  char_to_lower('A'): " + char_to_lower(c= "A"))
print("  char_to_lower('Z'): " + char_to_lower(c= "Z"))
print("  char_to_upper('5'): " + char_to_upper(c= "5"))
print("  char_to_lower('5'): " + char_to_lower(c= "5"))
print("")
print("String reversal:")
print("  reverse_characters('abc'): " + reverse_characters(s= "abc"))
print("  reverse_characters('hello'): " + reverse_characters(s= "hello"))
print("  reverse_characters('racecar'): " + reverse_characters(s= "racecar"))
print("  reverse_characters('12345'): " + reverse_characters(s= "12345"))
print("")
print("=== Practical Example: Title Case ===")
def to_title_case(s):
    if len(s) == 0:
        return s
    return char_to_upper(c= s[0]) + substring_end(s= string_to_lower(s= s), from_here= 1)

words = ["hello", "world", "lumen", "PROGRAMMING"]
i = 0
while i < len(words):
    word = words[i]
    print("  " + word + " -> " + to_title_case(s= word))
    i = i + 1
print("")
print("=== Practical Example: Palindrome Checker ===")
def is_palindrome(s):
    normalized = string_to_lower(s= s)
    return normalized == reverse_characters(s= normalized)

test_words = ["racecar", "hello", "madam", "world", "level"]
i = 0
while i < len(test_words):
    word = test_words[i]
    is_pal = is_palindrome(s= word)
    print("  '" + word + "' is palindrome: " + str(is_pal))
    i = i + 1
