def read_tuples():
    x = ()
    x = (1,)
    x = (1, 2,)
    x = 1, 2
    a, b = x
    (a, b) = x
    for a, b in x:
        pass
    return a, b
print("read tuples")
