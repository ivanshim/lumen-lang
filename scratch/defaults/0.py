def f(x=[]):
    x.append(1)
    return x
print(f())
print(f())
print(f([9]))
