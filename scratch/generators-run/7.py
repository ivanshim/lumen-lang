def g():
    yield 1
    yield 2
a, b = g()
print(a, b)
for a, b in [tuple(g()), tuple(g())]:
    print(a + b)
def counter():
    i = 0
    while True:
        yield i
        i += 1
it = counter()
print(any(it))
print(next(it))
print(list(x * 10 + y for x in range(3) if x > 0 for y in range(2)))
print([x * x for x in range(3)])
(a, b) = g()
print(a, b)
[a, b] = g()
print(a, b)
