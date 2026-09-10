def f(a, b=2):
    "doc"
    c = a + b
    return c
print(f.__name__, f.__qualname__, f.__module__, f.__doc__)
print(f.__code__.co_argcount, f.__code__.co_varnames, f.__defaults__)
f.tag = 8
print(f.__dict__["tag"])
def outer():
    def inner(): pass
    return inner
print(outer().__qualname__)
class C:
    v = 2
    m = staticmethod(f)
    def n(cls): return cls.__name__
    n = classmethod(n)
print(C.m(3), C().m(4), C.n(), C().n())
for name in dir(C):
    print(name)
