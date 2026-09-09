a = [0, 0]
i = 0
i, a[i] = 1, 9
print(i, a)
def once():
    print(7)
    return 8, 9
x = y = once()
print(x, y)
(a[0], a[1]) = a[1], a[0]
print(a)
