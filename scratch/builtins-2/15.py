def make():
    def f():
        return 1
    return f
a = make()
b = make()
c = a
print(id(a) != id(b), id(a) == id(c), a(), b())
