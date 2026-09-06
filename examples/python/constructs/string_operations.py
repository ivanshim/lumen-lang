import sys
# Ported from examples/lumen/constructs/string_operations.lm by scripts/port_examples.py; edit the Lumen original, not this file.
def char_at_or_null(s, index):
    if index < 0 or index >= len(s):
        return None
    return s[index]

print("=== String Operations Test ===")
str1 = "Hello"
str2 = " World"
result1 = str1 + str2
sys.stdout.write("Period operator (string . string): ")
print(result1)
num = 42
result2 = "Answer: " + num
sys.stdout.write("Period operator with number coercion: ")
print(result2)
x = 10
y = 20
result3 = "Sum: " + (x + y)
sys.stdout.write("Period operator with expression: ")
print(result3)
test_str = "Hello"
str_len = len(test_str)
sys.stdout.write("len('Hello'): ")
print(str_len)
utf8_str = "abc123"
utf8_len = len(utf8_str)
sys.stdout.write("len('abc123'): ")
print(utf8_len)
empty_str = ""
empty_len = len(empty_str)
sys.stdout.write("len(''): ")
print(empty_len)
arr = [1, 2, 3, 4, 5]
arr_len = len(arr)
sys.stdout.write("len([1,2,3,4,5]): ")
print(arr_len)
text = "Lumen"
ch0 = text[0]
sys.stdout.write("char_at('Lumen', 0): ")
print(ch0)
ch2 = text[2]
sys.stdout.write("char_at('Lumen', 2): ")
print(ch2)
ch4 = text[4]
sys.stdout.write("char_at('Lumen', 4): ")
print(ch4)
print("")
print("=== Testing char_at_or_null (permissive wrapper) ===")
ch_valid = char_at_or_null(text, 1)
sys.stdout.write("char_at_or_null('Lumen', 1): ")
print(ch_valid)
ch_oob = char_at_or_null(text, 10)
sys.stdout.write("char_at_or_null('Lumen', 10) [out of bounds]: ")
print(ch_oob)
ch_neg = char_at_or_null(text, -1)
sys.stdout.write("char_at_or_null('Lumen', -1) [negative]: ")
print(ch_neg)
ch_edge = char_at_or_null(text, 5)
sys.stdout.write("char_at_or_null('Lumen', 5) [at length]: ")
print(ch_edge)
word = "Test"
length = len(word)
first_char = word[0]
result10 = "Word: " + word + ", Length: " + length + ", First: " + first_char
sys.stdout.write("Combined operations: ")
print(result10)
print("Done!")
