a = {0: 0, 1: 1, 2: 1}
b = {1: 1, 2: 2, 3: 3}
c = a | b
print(len(c), c[2], c[3])
print(list(c))
print(a[2])
c |= {2: 9, 4: 4}
print(c[2], c[4])
print(list(c))
print(list(b | a))
print(5 | 2)
