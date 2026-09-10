import sys
print(sys.maxunicode, ord(chr(0x10ffff)), len("héllo"), "😀"[0])
print("ǅ".title(), "ΟΣ".lower(), "ß".capitalize(), "é".swapcase())
print("a".isidentifier(), "0".isidentifier(), "".isascii(), "é".isascii(), "¼".isnumeric())
print("abcd".translate(str.maketrans("abc", "xyz", "d")))
print("é".encode("ascii", errors="ignore"), "é".encode("ascii", errors="xmlcharrefreplace"))
print(repr(b"\xff".decode(errors="surrogateescape")), b"\xff".decode(errors="surrogateescape").encode(errors="surrogateescape"))
s = "a\ud800b"
print(len(s), repr(s[1]), repr(s[::-1]), ord(chr(0xdfff)))
print("%c" % 0x1f600, format(0x1f600, "c"), sorted(["😀", "é", "z"]))
