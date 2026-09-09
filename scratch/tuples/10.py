a, *b, c, d = [1, 2, 3]
print(a, b, c, d)
*a, = ()
print(a)
x, y = "ab"
print(x, y)
(a, (*b, c)) = (1, (2, 3, 4))
print(a, b, c)
