def outer(a):
    return lambda b=2: a + b
f = outer(5)
print(f(), f(b=3))
def make(x):
    return lambda a, /, b=2: x + a + b
f = make(10)
print(f(1), f(2, b=3))
def variadic(a):
    return lambda *xs: a + len(xs)
print(variadic(2)(1, 2, 3))
