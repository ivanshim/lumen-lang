def f(a, /, b=2, *rest, c=3, **kw):
    print(a, b, rest, c, len(kw))
f(1)
f(1, 4, 5, 6, c=7, x=8)
f(*[1], **{"b": 9, "c": 10, "x": 11})
f(b=8, *[1])
f(1, a=9)
print(*[1, 2], *[3])
