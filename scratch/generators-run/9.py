def g(start=2):
    while start < 5:
        yield start
        start += 1
a = g()
b = g(4)
print(next(a), next(b), next(a))
print(next(b, "done"))
print(list(a))
def expression():
    value = 10 + (yield 1)
    yield value
it = expression()
print(next(it))
print(it.send(7))
def loops():
    for x in range(4):
        if x == 1:
            continue
        for y in range(3):
            if y == 2:
                break
            yield x * 10 + y
    else:
        yield 99
print(list(loops()))
