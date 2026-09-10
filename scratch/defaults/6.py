def make(n):
    def f(x=[n]):
        x.append(0)
        return x
    return f
a = make(1)
b = make(2)
print(a())
print(b())
print(a())
