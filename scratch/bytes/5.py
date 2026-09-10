a = b"abc\x00\xff"
print(bytes(), bytes(3), a[-1], a[1:4], a[::-1])
print(97 in a, b"bc" in a, b"" in a, b"z" not in a)
print(b"a" < b"b", b"ab" <= b"ab", b"b" > b"a", 2 * b"xy")
print(str(b"x"), type(a), isinstance(a, bytes), isinstance("a", bytes))
d = {b"key": 7}
print(d[bytes([107, 101, 121])], hash(b"key") == hash(bytes([107, 101, 121])))
