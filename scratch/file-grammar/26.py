def expressions():
    l1 = lambda: 0
    l4 = lambda x = lambda y = lambda z=1: z: y(): x()
    l5 = lambda x, y, z=2: x + y + z
    l6 = lambda x, y, *, k=20: x+y+k
    l7 = lambda a, /, b=2, *args, k, **kwds: a
    l8 = lambda *args, **kwds,: 0
    @lambda f: f
    def decorated(): pass

print("read")
