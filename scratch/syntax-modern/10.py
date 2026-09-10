type Pair[K, V] = tuple[K, V]
def ident[T: (int, str), *Ts, **P](x):
    return x
print(ident(8))
def defaulted[T = int](x): return x
print(defaulted(8))
if False:
    with (
        open("a") as f,
        open("b") as g,
    ):
        print("unreached")
print((lambda x: lambda y: y + 1)(0)(3))
print((lambda x: x if x else 2)(0))
def enclosing():
    x = 6
    f = lambda: x
    x = 7
    return f
print(enclosing()())
print((lambda x: lambda y: x + y)(4)(5))
