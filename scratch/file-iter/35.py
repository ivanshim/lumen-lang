def f():
    for x in [1]:
        with manager():
            break
f()
