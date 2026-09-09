items = [4]
def f(x=items):
    x.append(5)
    return x
items.append(6)
a = f()
print(a)
a.append(7)
print(f())
def reset(x=[]):
    print(len(x))
    x = [9]
    return x
print(reset())
print(reset())
