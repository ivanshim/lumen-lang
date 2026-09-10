def f():
    print("body must wait")
    if False:
        yield 1
f()
