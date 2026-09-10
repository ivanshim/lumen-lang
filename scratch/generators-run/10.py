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
def after_send():
    value = yield 0
    yield from inner()
    yield value
it = after_send()
print(next(it))
print(it.send(6))
print(it.send(7))
print(next(it))
it = inner()
print(list(it))
def exhausted():
    yield (yield from it)
print(list(exhausted()))
