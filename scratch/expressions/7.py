def make(x):
    return lambda: x
f = make(3)
print(f())
