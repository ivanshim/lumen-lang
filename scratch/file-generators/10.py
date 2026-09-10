def outer():
    value = 1
    def inner():
        nonlocal value
        value = 2
    inner()

outer()
