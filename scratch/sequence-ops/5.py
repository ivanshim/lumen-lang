a = [1]
b = a
a += (2, 3)
print(a, b)
a = a + [4]
print(a, b)
b *= 2
print(a, b)
t = (1, 2)
u = t
t += (3,)
t *= 2
print(t, u)
inner = [2]
outer = [(1, inner), inner]
inner.append(3)
print(outer)
