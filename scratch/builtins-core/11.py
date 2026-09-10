print(repr(pow(4, 0.5)), repr(pow(2, -2)))
a = [1]
b = a
print(id(a) == id(b), id([1]) == id(a))
print(type(1) == int, type(()) == tuple, hash((True, "x")) == hash((1, "x")))
print(len(set([True, 1, 1.0, None, None])))
