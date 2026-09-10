def g():
    yield 1
    yield 2
it = g()
print(next(it))
print(next(it))
next(it)
