def f():
    x = 1
    def inner():
        nonlocal x
        x = 2
    inner()
f()
