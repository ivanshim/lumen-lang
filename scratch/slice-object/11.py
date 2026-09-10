class G:
    def __setitem__(self, key, value):
        print(key, value)
def bound():
    print("bound")
    return 2
def value():
    print("value")
    return 7
g = G()
g[1:bound(), 3] = value()
l = list(range(6))
del l[::-2]
print(l)
l[:] = range(3)
print(l)
