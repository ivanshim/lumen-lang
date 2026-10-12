import gc
import weakref

def plain():
    return 1

def documented():
    'original documentation'
    return 2

assert plain.__doc__ is None
assert plain.__dict__ == {}
plain.__code__ = documented.__code__
assert plain() == 2 and plain.__doc__ is None
plain.__doc__ = 'written documentation'
assert plain.__doc__ == 'written documentation'
assert plain.__dict__ == {}
del plain.__doc__
assert plain.__doc__ is None
assert documented.__doc__ is documented.__doc__

def factory(value):
    def inner():
        return value
    return inner

left, right = factory(1), factory(2)
assert left.__doc__ is None and right.__doc__ is None
left.__doc__ = 'left'
assert left.__doc__ == 'left' and right.__doc__ is None
reference = weakref.ref(left)
del left
gc.collect()
assert reference() is None
print('function documentation preserves writes, code replacement and collection')
