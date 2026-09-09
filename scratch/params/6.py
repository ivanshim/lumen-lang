def f(a=1, /, b=2, *, c=3,):
    print(a, b, c)
f()
f(c=9)
f(4, b=5)
def g(*args,):
    print(len(args))
def h(**kw,):
    print(len(kw))
g()
h()
h(**{"x": 1}, **{"y": 2})
def letters(a, b):
    print(a, b)
letters(*"xy")
letters(*{"x": 1, "y": 2})
