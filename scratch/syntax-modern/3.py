print((lambda *a, **k: 0)(1, 2, z=3))
print((lambda x=1, /, y=2, *, z=3: x)())
def apply(f=lambda x: x + 1):
    return f(4)
print(apply())
f = (lambda x: x + 2) if True else (lambda x: x + 3)
print(f(5))
