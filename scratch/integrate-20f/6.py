from contextlib import contextmanager
class Base:
    pass
class Expanded(*(Base,), **{'metaclass': type}):
    pass
obj = Expanded()
namespace = {'value': 1}
obj.__dict__ = namespace
obj.value = 2
assert namespace['value'] == 2
obj.__dict__.update(extra=3)
assert obj.extra == 3
saved_namespace = obj.__dict__
class Replacement(Base):
    pass
obj.__class__ = Replacement
assert obj.__dict__ is saved_namespace
assert obj.value == 2
class CallableReplacement(Base):
    def __call__(self):
        return 42
obj.__class__ = CallableReplacement
assert obj() == 42
finished = []
@contextmanager
def managed():
    try:
        yield [1, 2, 3]
    finally:
        finished.append('done')
with managed() as [first, *rest]:
    assert first == 1 and rest == [2, 3]
assert finished == ['done']
def generator():
    try:
        yield 1
    finally:
        finished.append('closed')
g = generator()
assert next(g) == 1
close = g.close
close()
assert finished == ['done', 'closed']
def function(value: int = 1):
    return value
saved_code = function.__code__
function.__dict__.update(flag=3)
assert function.flag == 3
assert function.__code__ is saved_code
assert function.__annotations__ == {'value': int}
print('class expansion, namespace identity, cleanup and metadata preserved')
