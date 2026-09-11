# A generic routine head is read and the routine runs, as CPython 3.12 runs it.
def generic[T](x: T) -> T:
    return x
print(generic(5))
