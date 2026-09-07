# Ported from examples/lumen/libraries/test_string_transform.lm by scripts/port_examples.py; edit the Lumen original, not this file.
puts("=== String Case Transformation ===")
puts("")
puts("Uppercase conversion:")
puts("  string_to_upper('hello'): " + string_to_upper("hello"))
puts("  string_to_upper('world'): " + string_to_upper("world"))
puts("  string_to_upper('Hello123'): " + string_to_upper("Hello123"))
puts("")
puts("Lowercase conversion:")
puts("  string_to_lower('HELLO'): " + string_to_lower("HELLO"))
puts("  string_to_lower('WORLD'): " + string_to_lower("WORLD"))
puts("  string_to_lower('Hello123'): " + string_to_lower("Hello123"))
puts("")
puts("Single character transformations:")
puts("  char_to_upper('a'): " + char_to_upper("a"))
puts("  char_to_upper('z'): " + char_to_upper("z"))
puts("  char_to_lower('A'): " + char_to_lower("A"))
puts("  char_to_lower('Z'): " + char_to_lower("Z"))
puts("  char_to_upper('5'): " + char_to_upper("5"))
puts("  char_to_lower('5'): " + char_to_lower("5"))
puts("")
puts("String reversal:")
puts("  reverse_characters('abc'): " + reverse_characters("abc"))
puts("  reverse_characters('hello'): " + reverse_characters("hello"))
puts("  reverse_characters('racecar'): " + reverse_characters("racecar"))
puts("  reverse_characters('12345'): " + reverse_characters("12345"))
puts("")
puts("=== Practical Example: Title Case ===")
def to_title_case(s)
    if s.length == 0 then
        return s
    end
    return char_to_upper(s[0]) + substring_end(string_to_lower(s), 1)
end

words = ["hello", "world", "lumen", "PROGRAMMING"]
i = 0
while i < words.length do
    word = words[i]
    puts("  " + word + " -> " + to_title_case(word))
    i = i + 1
end
puts("")
puts("=== Practical Example: Palindrome Checker ===")
def is_palindrome(s)
    normalized = string_to_lower(s)
    return normalized == reverse_characters(normalized)
end

test_words = ["racecar", "hello", "madam", "world", "level"]
i = 0
while i < test_words.length do
    word = test_words[i]
    is_pal = is_palindrome(word)
    puts("  '" + word + "' is palindrome: " + is_pal.to_s)
    i = i + 1
end
