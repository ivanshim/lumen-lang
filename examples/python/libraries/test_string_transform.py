# Ported from examples/lumen/libraries/test_string_transform.lm by scripts/port_examples.py; edit the Lumen original, not this file.
def substring(s, from_start, to_end):
    index = from_start
    out = ""
    while index < to_end:
        out = out + s[index]
        index = index + 1
    return out

def substring_end(s, from_here):
    return substring(s, from_here, len(s))

def char_to_upper(c):
    o = ord(c)
    if o >= ord("a") and o <= ord("z"):
        return chr(o - 32)
    else:
        return c

def char_to_lower(c):
    o = ord(c)
    if o >= ord("A") and o <= ord("Z"):
        return chr(o + 32)
    else:
        return c

def string_to_upper(s):
    result = ""
    i = 0
    while i < len(s):
        result = result + char_to_upper(s[i])
        i = i + 1
    return result

def string_to_lower(s):
    result = ""
    i = 0
    while i < len(s):
        result = result + char_to_lower(s[i])
        i = i + 1
    return result

def reverse_characters(s):
    result = ""
    index = len(s) - 1
    while index >= 0:
        result = result + s[index]
        index = index - 1
    return result

print("=== String Case Transformation ===")
print("")
print("Uppercase conversion:")
print("  string_to_upper('hello'): " + string_to_upper("hello"))
print("  string_to_upper('world'): " + string_to_upper("world"))
print("  string_to_upper('Hello123'): " + string_to_upper("Hello123"))
print("")
print("Lowercase conversion:")
print("  string_to_lower('HELLO'): " + string_to_lower("HELLO"))
print("  string_to_lower('WORLD'): " + string_to_lower("WORLD"))
print("  string_to_lower('Hello123'): " + string_to_lower("Hello123"))
print("")
print("Single character transformations:")
print("  char_to_upper('a'): " + char_to_upper("a"))
print("  char_to_upper('z'): " + char_to_upper("z"))
print("  char_to_lower('A'): " + char_to_lower("A"))
print("  char_to_lower('Z'): " + char_to_lower("Z"))
print("  char_to_upper('5'): " + char_to_upper("5"))
print("  char_to_lower('5'): " + char_to_lower("5"))
print("")
print("String reversal:")
print("  reverse_characters('abc'): " + reverse_characters("abc"))
print("  reverse_characters('hello'): " + reverse_characters("hello"))
print("  reverse_characters('racecar'): " + reverse_characters("racecar"))
print("  reverse_characters('12345'): " + reverse_characters("12345"))
print("")
print("=== Practical Example: Title Case ===")
def to_title_case(s):
    if len(s) == 0:
        return s
    return char_to_upper(s[0]) + substring_end(string_to_lower(s), 1)

words = ["hello", "world", "lumen", "PROGRAMMING"]
i = 0
while i < len(words):
    word = words[i]
    print("  " + word + " -> " + to_title_case(word))
    i = i + 1
print("")
print("=== Practical Example: Palindrome Checker ===")
def is_palindrome(s):
    normalized = string_to_lower(s)
    return normalized == reverse_characters(normalized)

test_words = ["racecar", "hello", "madam", "world", "level"]
i = 0
while i < len(test_words):
    word = test_words[i]
    is_pal = is_palindrome(word)
    print("  '" + word + "' is palindrome: " + str(is_pal))
    i = i + 1
