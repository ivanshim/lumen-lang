def f(x):
    def inc():
        nonlocal x
        x += 1
        return x
    return inc()
f(0)
