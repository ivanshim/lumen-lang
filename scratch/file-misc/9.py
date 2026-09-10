def f(a, /, b=2, *, c=3):
    return a + b + c
print(f(1, c=4))
