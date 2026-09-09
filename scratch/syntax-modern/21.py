def unchanged(f):
    return f
@unchanged
def ident[T](x: T) -> T:
    return x
print(ident(7))
