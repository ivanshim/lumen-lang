import types

def factory(value):
    def inner():
        return value
    return inner

left, right = factory(1), factory(2)
name, qualified = left.__name__, left.__qualname__
assert name is left.__name__ and qualified is left.__qualname__
assert left.__dict__ == {}
left.__name__ = 'changed'
left.__qualname__ = 'outer.changed'
assert left.__name__ == 'changed' and right.__name__ == 'inner'
assert left.__qualname__ == 'outer.changed'
assert right.__qualname__ == qualified
assert left.__dict__ == {}

def plain():
    return 1

def other():
    return 2

name, qualified = plain.__name__, plain.__qualname__
plain.__code__ = other.__code__
assert plain() == 2
assert plain.__name__ is name and plain.__qualname__ is qualified
made = types.FunctionType(other.__code__, globals(), 'supplied')
assert made.__name__ == 'supplied' and made() == 2
assert other.__name__ == 'other'
print('function names preserve identity, overrides and code replacement')
