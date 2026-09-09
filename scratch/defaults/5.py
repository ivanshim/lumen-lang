def make(n):
    print(n)
    return []
def f(a=make(1), b=make(2)):
    a.append(3)
    return a
print(f())
print(f())
k = lambda x=make(4): x
a = k()
a.append(5)
print(k())
l = lambda x = lambda y = lambda z=1: z: y(): x()
print(l())
