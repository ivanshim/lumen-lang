a, b = {1: 2}, {3: 4}
print(a[1], b[3])
a, b = b, a
print(a[3], b[1])
x, y = {1: 2, 3: 4}
print(x, y)
x, = (9,)
print(x)
def pair():
    print("once")
    return 5, 6
x, y = pair()
print(x, y)
