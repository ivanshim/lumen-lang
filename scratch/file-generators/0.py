def generator1():
    return (yield from generator2())

def generator2():
    yield 1

print('read')
