def inner():
    value = yield 1
    yield value
    return 9
def outer():
    result = yield from inner()
    yield result
it = outer()
print(next(it))
print(it.send(8))
print(next(it))
print(next(it, "done"))
it = outer()
print(next(it))
print(it.close())
print(next(it, "closed"))
def items():
    yield None, True, "a"
    yield (2,)
    yield ()
for item in items():
    print(item)
