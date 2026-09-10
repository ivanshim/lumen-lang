def outer():
    x = 1
    def inner():
        return x
    del x
    x = 3
    return inner
print(outer()())
def make(x):
    def read():
        return x
    def drop():
        nonlocal x
        del x
    def set_value():
        nonlocal x
        x = 8
    drop()
    set_value()
    return read
print(make(2)())
