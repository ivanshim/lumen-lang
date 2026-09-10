def outer():
    def inner():
        return x
    inner()
    x = 1
outer()
