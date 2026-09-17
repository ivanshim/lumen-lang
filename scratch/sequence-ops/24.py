print(["hello", ("world",), {"a": ["b"]}, {"x", "x"}])
print(str(("a",)), str({"a": "b"}))
try:
    print("Answer: " + 42, 42 + "!", "a" + 1)
except TypeError as e:
    print(str(e))
text = "value="
try:
    text += 7
except TypeError as e:
    print(str(e))
print(text)
letters = []
letters += "ab"
print(letters)
