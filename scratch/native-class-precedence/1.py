import typing
import unittest
assert repr(typing.Any) == 'typing.Any'
for cls in (str, list, int, dict, type):
    assert cls.__class__ is type
case = unittest.TestCase()
with case.assertRaises(TypeError):
    isinstance(42, typing.Any)
for cls, name in ((str, '__add__'), (str, '__getitem__'), (list, '__getitem__'), (list, 'sort'), (type, 'mro')):
    assert callable(getattr(cls, name))
assert str.__add__('a', 'b') == 'ab'
assert list.__getitem__([7], 0) == 7
assert typing.no_type_check(dict) is dict
print('native class methods precede inherited root defaults')
