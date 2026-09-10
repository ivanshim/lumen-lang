x = 99
def outer():
    x = 2
    def inner():
        yield x
    return inner()
next(outer())
