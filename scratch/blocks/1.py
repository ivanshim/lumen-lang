a = [1, 2, 3]
del a[1]
print(a)
v = 1
def f():
    global v
    v = 9
f()
print(v)
x = 4
del x
print(x)
