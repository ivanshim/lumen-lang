a = 3
del a
a = 4
print(a)
def local():
    b = 5
    del b
    b = 6
    return b
print(local())
