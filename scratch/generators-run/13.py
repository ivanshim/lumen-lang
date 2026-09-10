def g():
    yield next(it)
it = g()
next(it)
