def outer():
    x = 1
    f = lambda: x
    del x
    x = 9
    return f
print(outer()())
