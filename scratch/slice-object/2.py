l = list(range(6))
print(l[slice(1, 4)], l[slice(None, None, -2)])
del l[::2]
print(l)
l[1:3] = [9]
print(l)
