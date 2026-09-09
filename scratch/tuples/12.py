def spread():
    z = [2, 3]
    return 1, *z
print(spread())
a = (0, *[1, 2], 3, *[4, 5],)
print(a)
b = *[6, 7], 8
print(b)
print(len((*[],)))
