type Pair[K, V] = tuple[K, V]
def ident[T: (int, str) = int, *Ts, **P](x):
    return x
print(ident(8))
if False:
    with (
        open("a") as f,
        open("b") as g,
    ):
        print("unreached")
    nonlocal missing
print((lambda x: lambda y: y + 1)(0)(3))
print((lambda x: x if x else 2)(0))
