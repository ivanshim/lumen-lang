a = bytearray(b"ab")
b = a
a += b"c"
print(a, b)
a *= 2
print(a, b)
c = a + b"!"
print(a, c)
a *= 0
print(a, b)
x = 3
x += 2
x *= 4
print(x)
