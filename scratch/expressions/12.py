f = lambda a,: 0
f = lambda *args,: 0
f = lambda **kwds,: 0
f = lambda a, *args,: 0
f = lambda a, **kwds,: 0
f = lambda *args, b,: 0
f = lambda *, b,: 0
f = lambda *args, **kwds,: 0
f = lambda a, *args, b,: 0
f = lambda a, *, b,: 0
f = lambda a, *args, **kwds,: 0
f = lambda *args, b, **kwds,: 0
f = lambda *, b, **kwds,: 0
f = lambda a, *args, b, **kwds,: 0
f = lambda a, *, b, **kwds,: 0
print("read")
def outer(x):
    return lambda: (lambda x: x)(2)
f = outer(5)
print(f())
def guarded(x):
    return lambda: 1 if True else x
print(guarded(8)())
print("a" in {"a": 1}, "b" not in {"a": 1})
def local(x):
    return lambda: (x := 4)
print(local(3)())
