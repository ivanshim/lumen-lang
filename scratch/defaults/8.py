inner = []
def f(x=[inner]):
    return x
a = f()
b = a[0]
b.append(1)
print(f())
a[0] = [2]
print(inner)
print(f())
def g(d={"k": inner}):
    return d
a = g()
a["k"] = [3]
print(inner)
print(g()["k"])
