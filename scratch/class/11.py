class L(list):
    pass

a = L([1, 2])
alias = a
a[:] = a
print('self', a, a is alias, type(a).__name__)
a[:] = (item * 2 for item in a)
print('generator', a, a is alias)

class Reading:
    def __iter__(self):
        for item in a:
            yield item + 1

a[:] = Reading()
print('iterable', a, a is alias)
a = L([1, 2, 3, 4])
a[::2] = (item * 10 for item in a[::2])
print('extended', a)
try:
    a[::2] = a
except ValueError as error:
    print('length', type(error).__name__, str(error), a)

def broken():
    yield 99
    raise RuntimeError('iterator stopped')

try:
    a[:] = broken()
except RuntimeError as error:
    print('exception', type(error).__name__, str(error), a)
