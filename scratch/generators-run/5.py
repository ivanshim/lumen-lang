def g():
    print("must not run")
    yield 1
g().send(3)
