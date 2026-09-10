x = 10
def outer():
    x = 1
    f = lambda: x
    del x
    return f
print(outer()())
