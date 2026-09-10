def make(x):
    def f():
        return x
    return f
a = make(1)
b = make(2)
c = a
print(id(a) != id(b), id(a) == id(c), a(), b())
