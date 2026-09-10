a = [1, 2]
b = a
f = a.append
f(3)
a = [9]
f(4)
print(a, b)
b.insert(-1, 8)
b.remove(2)
b.reverse()
c = b.copy()
b.clear()
print(b, c, c.count(1), c.index(8))
