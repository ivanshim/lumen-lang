class A: pass
def f(a, b=2): "doc"
print(f.__name__, f.__doc__, f.__defaults__)
f.tag = 1
print(f.tag)
T = type("T", (A,), {"v": 3})
print(T().v, T.__bases__)
