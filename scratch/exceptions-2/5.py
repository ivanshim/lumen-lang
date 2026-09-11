def inner():
    raise ValueError("x")
def outer():
    inner()
outer()
