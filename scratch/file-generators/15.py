def forms():
    yield
    value = yield 1
    yield 1, 2, *[3]
    value = (yield (1, 2))
    accept((yield 3), (yield from ()))
    return (yield from [1, 2])

print('read')
