import sys
# Ported from examples/lumen/string_library_demo.lm by scripts/port_examples.py; edit the Lumen original, not this file.
def substring(s, from_start, to_end):
    index = from_start
    out = ""
    while index < to_end:
        out = out + s[index]
        index = index + 1
    return out

def substring_end(s, from_here):
    return substring(s, from_here, len(s))

def substring_start(s, to_here):
    return substring(s, 0, to_here)

def starts_with(s, prefix):
    return len(prefix) <= len(s) and substring(s, 0, len(prefix)) == prefix

def ends_with(s, suffix):
    return len(suffix) <= len(s) and substring(s, len(s) - len(suffix), len(s)) == suffix

def repeat_string(s, repetitions):
    out = ""
    i = 0
    while i < repetitions:
        out = out + s
        i = i + 1
    return out

def join_strings(arr, separator):
    out = ""
    n = len(arr)
    i = 0
    while i < n:
        if i > 0:
            out = out + separator
        out = out + arr[i]
        i = i + 1
    return out

def index_of(s, needle):
    n = len(needle)
    i = 0
    while i + n <= len(s):
        if substring(s, i, i + n) == needle:
            return i
        i = i + 1
    return -1

def has_substring(s, needle):
    return index_of(s, needle) >= 0

print("=== String Library Examples ===")
print("")
text = "Hello World"
sys.stdout.write("Original: ")
print(text)
sys.stdout.write("substring(text, 0, 5): ")
print(substring(text, 0, 5))
sys.stdout.write("substring(text, 6, 11): ")
print(substring(text, 6, 11))
print("")
sys.stdout.write("substring_end(text, 6): ")
print(substring_end(text, 6))
print("")
sys.stdout.write("substring_start(text, 5): ")
print(substring_start(text, 5))
print("")
sys.stdout.write("starts_with('Hello World', 'Hello'): ")
print(starts_with(text, "Hello"))
sys.stdout.write("starts_with('Hello World', 'World'): ")
print(starts_with(text, "World"))
print("")
sys.stdout.write("ends_with('Hello World', 'World'): ")
print(ends_with(text, "World"))
sys.stdout.write("ends_with('Hello World', 'Hello'): ")
print(ends_with(text, "Hello"))
print("")
sys.stdout.write("repeat_string('Ha', 5): ")
print(repeat_string("Ha", 5))
sys.stdout.write("repeat_string('-=', 10): ")
print(repeat_string("-=", 10))
print("")
fruits = ["apple", "banana", "cherry"]
sys.stdout.write("join_strings(['apple', 'banana', 'cherry'], ', '): ")
print(join_strings(fruits, ", "))
sys.stdout.write("join_strings(['apple', 'banana', 'cherry'], ' | '): ")
print(join_strings(fruits, " | "))
print("")
sentence = "The quick brown fox jumps over the lazy dog"
sys.stdout.write("index_of('The quick brown fox...', 'fox'): ")
print(index_of(sentence, "fox"))
sys.stdout.write("index_of('The quick brown fox...', 'cat'): ")
print(index_of(sentence, "cat"))
print("")
sys.stdout.write("has_substring('The quick brown fox...', 'quick'): ")
print(has_substring(sentence, "quick"))
sys.stdout.write("has_substring('The quick brown fox...', 'slow'): ")
print(has_substring(sentence, "slow"))
print("")
print("=== Practical Example ===")
name = "Lumen"
version = "1.0"
description = "A minimal language"
separator = repeat_string("-", 40)
print(separator)
info = "Project: " + name
print(info)
info2 = "Version: " + version
print(info2)
info3 = "Description: " + description
print(info3)
print(separator)
