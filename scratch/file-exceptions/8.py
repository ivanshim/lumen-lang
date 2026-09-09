def read_yields():
    yield
    yield 1
    yield from source
    x = (yield 2)
read_yields()
