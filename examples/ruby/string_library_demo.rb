# Ported from examples/lumen/string_library_demo.lm by scripts/port_examples.py; edit the Lumen original, not this file.
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

def substring_start(s, to_here)
    return substring(s, 0, to_here)
end

def starts_with(s, prefix)
    return prefix.length <= s.length && substring(s, 0, prefix.length) == prefix
end

def ends_with(s, suffix)
    return suffix.length <= s.length && substring(s, s.length - suffix.length, s.length) == suffix
end

def repeat_string(s, repetitions)
    out = ""
    i = 0
    while i < repetitions do
        out = out + s
        i = i + 1
    end
    return out
end

def join_strings(arr, separator)
    out = ""
    n = arr.length
    i = 0
    while i < n do
        if i > 0 then
            out = out + separator
        end
        out = out + arr[i]
        i = i + 1
    end
    return out
end

def index_of(s, needle)
    n = needle.length
    i = 0
    while i + n <= s.length do
        if substring(s, i, i + n) == needle then
            return i
        end
        i = i + 1
    end
    return -1
end

def has_substring(s, needle)
    return index_of(s, needle) >= 0
end

puts("=== String Library Examples ===")
puts("")
text = "Hello World"
print("Original: ")
puts(text)
print("substring(text, 0, 5): ")
puts(substring(text, 0, 5))
print("substring(text, 6, 11): ")
puts(substring(text, 6, 11))
puts("")
print("substring_end(text, 6): ")
puts(substring_end(text, 6))
puts("")
print("substring_start(text, 5): ")
puts(substring_start(text, 5))
puts("")
print("starts_with('Hello World', 'Hello'): ")
puts(starts_with(text, "Hello"))
print("starts_with('Hello World', 'World'): ")
puts(starts_with(text, "World"))
puts("")
print("ends_with('Hello World', 'World'): ")
puts(ends_with(text, "World"))
print("ends_with('Hello World', 'Hello'): ")
puts(ends_with(text, "Hello"))
puts("")
print("repeat_string('Ha', 5): ")
puts(repeat_string("Ha", 5))
print("repeat_string('-=', 10): ")
puts(repeat_string("-=", 10))
puts("")
fruits = ["apple", "banana", "cherry"]
print("join_strings(['apple', 'banana', 'cherry'], ', '): ")
puts(join_strings(fruits, ", "))
print("join_strings(['apple', 'banana', 'cherry'], ' | '): ")
puts(join_strings(fruits, " | "))
puts("")
sentence = "The quick brown fox jumps over the lazy dog"
print("index_of('The quick brown fox...', 'fox'): ")
puts(index_of(sentence, "fox"))
print("index_of('The quick brown fox...', 'cat'): ")
puts(index_of(sentence, "cat"))
puts("")
print("has_substring('The quick brown fox...', 'quick'): ")
puts(has_substring(sentence, "quick"))
print("has_substring('The quick brown fox...', 'slow'): ")
puts(has_substring(sentence, "slow"))
puts("")
puts("=== Practical Example ===")
name = "Lumen"
version = "1.0"
description = "A minimal language"
separator = repeat_string("-", 40)
puts(separator)
info = "Project: " + name
puts(info)
info2 = "Version: " + version
puts(info2)
info3 = "Description: " + description
puts(info3)
puts(separator)
