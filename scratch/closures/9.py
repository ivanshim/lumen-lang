def outer():
    def inner():
        return x
    print(x)
    x = 2
outer()
