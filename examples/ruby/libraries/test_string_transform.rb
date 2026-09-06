# Ported from examples/lumen/libraries/test_string_transform.lm by scripts/port_examples.py; edit the Lumen original, not this file.
def substring(s, from_start, to_end)
    index = from_start
    out = ""
    while index < to_end do
        out = out + s[index]
        index = index + 1
    end
    return out
end

def substring_end(s, from_here)
    return substring(s, from_here, s.length)
end

def char_to_upper(c)
    o = c.ord
    if o >= "a".ord && o <= "z".ord then
        return (o - 32).chr
    else
        return c
    end
end

def char_to_lower(c)
    o = c.ord
    if o >= "A".ord && o <= "Z".ord then
        return (o + 32).chr
    else
        return c
    end
end

def string_to_upper(s)
    result = ""
    i = 0
    while i < s.length do
        result = result + char_to_upper(s[i])
        i = i + 1
    end
    return result
end

def string_to_lower(s)
    result = ""
    i = 0
    while i < s.length do
        result = result + char_to_lower(s[i])
        i = i + 1
    end
    return result
end

def reverse_characters(s)
    result = ""
    index = s.length - 1
    while index >= 0 do
        result = result + s[index]
        index = index - 1
    end
    return result
end

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
