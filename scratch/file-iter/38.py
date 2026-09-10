def outer():
    def inner():
        yield 1
    return 2
print(outer())
