def f(x=[]): x.append(1); return x
print(f()); print(f())
items = [1, 2, 3]
def g(n=len(items)):
    return n
items.append(4)
print(g())
