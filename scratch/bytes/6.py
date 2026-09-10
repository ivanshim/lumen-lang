a = bytearray(b"abc")
b = a
b[0] = 90
print(a, b, a == b"Zbc", type(a) is bytearray, isinstance(a, bytearray))
c = a[1:]
c[0] = 88
print(a, c, bytes(a))
a[1:2] = b"12"
print(b)
a[::2] = b"xy"
print(b)
a.append(33)
print(b)
