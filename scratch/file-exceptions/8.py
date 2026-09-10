def read_yields():
    print("body must wait")
    yield
    yield 1
    yield from source
    x = (yield 2)
read_yields()
