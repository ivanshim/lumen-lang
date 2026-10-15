import types
import typing
for descriptor in (object.__init__, object.__str__, object.__lt__, int.__lt__):
    assert isinstance(descriptor, types.WrapperDescriptorType)
for name in ('__delattr__', '__getattribute__', '__setattr__', '__init_subclass__'):
    assert callable(getattr(dict, name))
class UnhashableMeta(type):
    __hash__ = None
class C(metaclass=UnhashableMeta):
    pass
try:
    hash(C)
except TypeError as error:
    assert str(error) == "unhashable type: 'UnhashableMeta'"
else:
    raise AssertionError('unhashable metaclass accepted')
T = typing.TypeVar('T')
class Y(typing.Generic[T], typing.NamedTuple):
    x: T
assert Y[int](3).x == 3
try:
    object.__new__(Y)
except TypeError:
    pass
else:
    raise AssertionError('opaque tuple layout allocated through object')
print('native descriptors and alternate base order agree')
