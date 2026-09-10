def outer():
    x = 1
    def inner():
        nonlocal x
    inner()
outer()
