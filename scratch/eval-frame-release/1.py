import unittest
case = unittest.TestCase()
for expression in ("int('invalid')", "1 / 0", "missing_name"):
    with case.assertRaises(Exception):
        eval(expression, {})
    assert type(1) is int
    assert type(None).__class__ is type
    assert callable(dict.__getitem__)
    assert callable(str.__add__)
    assert dict.__getitem__({'value': 7}, 'value') == 7
    assert str.__add__('a', 'b') == 'ab'
print('cleared eval frames preserve builtin namespaces')
