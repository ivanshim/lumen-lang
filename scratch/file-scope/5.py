def g(y):
    print("body must wait")
    for i in range(y):
        yield i
g(2)
