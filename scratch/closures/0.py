def outer():
    x = 1
    def inner():
        return x + 1
    return inner()
print(outer())
