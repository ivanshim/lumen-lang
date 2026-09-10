splitter = " a  b ".split
print(splitter(maxsplit=1))
for word in "a b".split():
    print(word)
a, b = "a=b".partition("=")[::2]
print(a, b, "b" in "a b".split())
print("{word!a}".format_map({"word": "é"}))
print("a\nb".splitlines(keepends="yes"), "a\nb".splitlines(None))
print("\x00\x01z".translate(["a", None]), "-".join(range(0)))
