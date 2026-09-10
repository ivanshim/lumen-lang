def g():
    yield 1
    yield 2
def outer():
    yield 0
    yield from g()
    yield 3
for v in outer():
    print(v)
print(sum(x * x for x in range(4)))
