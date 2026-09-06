# Ported from examples/lumen/constructs/string_operations.lm by scripts/port_examples.py; edit the Lumen original, not this file.
def char_at_or_null(s, index)
    if index < 0 || index >= s.length then
        return nil
    end
    return s[index]
end

puts("=== String Operations Test ===")
str1 = "Hello"
str2 = " World"
result1 = str1 + str2
print("Period operator (string . string): ")
puts(result1)
num = 42
result2 = "Answer: " + num
print("Period operator with number coercion: ")
puts(result2)
x = 10
y = 20
result3 = "Sum: " + (x + y)
print("Period operator with expression: ")
puts(result3)
test_str = "Hello"
str_len = test_str.length
print("len('Hello'): ")
puts(str_len)
utf8_str = "abc123"
utf8_len = utf8_str.length
print("len('abc123'): ")
puts(utf8_len)
empty_str = ""
empty_len = empty_str.length
print("len(''): ")
puts(empty_len)
arr = [1, 2, 3, 4, 5]
arr_len = arr.length
print("len([1,2,3,4,5]): ")
puts(arr_len)
text = "Lumen"
ch0 = text[0]
print("char_at('Lumen', 0): ")
puts(ch0)
ch2 = text[2]
print("char_at('Lumen', 2): ")
puts(ch2)
ch4 = text[4]
print("char_at('Lumen', 4): ")
puts(ch4)
puts("")
puts("=== Testing char_at_or_null (permissive wrapper) ===")
ch_valid = char_at_or_null(text, 1)
print("char_at_or_null('Lumen', 1): ")
puts(ch_valid)
ch_oob = char_at_or_null(text, 10)
print("char_at_or_null('Lumen', 10) [out of bounds]: ")
puts(ch_oob)
ch_neg = char_at_or_null(text, -1)
print("char_at_or_null('Lumen', -1) [negative]: ")
puts(ch_neg)
ch_edge = char_at_or_null(text, 5)
print("char_at_or_null('Lumen', 5) [at length]: ")
puts(ch_edge)
word = "Test"
length_ = word.length
first_char = word[0]
result10 = "Word: " + word + ", Length: " + length_ + ", First: " + first_char
print("Combined operations: ")
puts(result10)
puts("Done!")
