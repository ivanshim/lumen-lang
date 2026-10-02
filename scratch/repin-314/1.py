import sys
import platform
import builtins
import math
assert sys.version_info == (3, 14, 8, 'final', 0)
assert sys.version.startswith('3.14.8 ')
assert sys.hexversion == 0x030e08f0
assert platform.python_version() == '3.14.8'
assert not hasattr(builtins, 'sentinel')
assert not hasattr(builtins, 'frozendict')
for name in ('acospi', 'asinpi', 'atan2pi', 'atanpi', 'cospi', 'fmax', 'fmin', 'isnormal', 'issubnormal', 'signbit', 'sinpi', 'tanpi'):
    assert not hasattr(math, name)
for name in ('stop_value', 'stop_exception'):
    try:
        iter(lambda: 1, **{name: ValueError})
    except TypeError:
        pass
    else:
        raise AssertionError(name)
def generator():
    yield 1
    yield 2
assert not hasattr(generator(), 'gi_state')
assert 'gi_state' not in dir(generator())
assert (lambda a, b: a + b)(*generator()) == 3
try:
    compile('def f(a, a): pass', '<probe>', 'exec')
except SyntaxError as error:
    assert 'duplicate argument' in str(error)
else:
    raise AssertionError('duplicate argument accepted')
assert bytes in {bytes, super}
assert super in {bytes, super}
for source in ('[*x for x in xs]', '{*x for x in xs}', '{**x for x in xs}'):
    try:
        compile(source, '<probe>', 'exec')
    except SyntaxError as error:
        assert 'unpacking' in str(error)
    else:
        raise AssertionError(source)
class Indexable:
    def __index__(self):
        return 5
assert math.comb(Indexable(), 2) == 10
assert math.perm(Indexable(), 2) == 20
assert math.factorial(Indexable()) == 120
try:
    math.factorial(-1.0)
except TypeError:
    pass
else:
    raise AssertionError('factorial accepted a float')
import copy
x = {}
x['self'] = x
y = copy.deepcopy(x)
assert y is not x and y['self'] is y
assert {} is not []
compile('from . lazy import value', '<probe>', 'exec')
class PseudoFloat:
    def __float__(self):
        return 3.14
try:
    '%x' % PseudoFloat()
except TypeError as error:
    assert str(error) == '%x format: an integer is required, not PseudoFloat'
else:
    raise AssertionError('hex formatting accepted a float protocol')
try:
    '%u' % 3j
except TypeError as error:
    assert str(error) == '%u format: a real number is required, not complex'
else:
    raise AssertionError('decimal formatting accepted complex')
try:
    '%c' % PseudoFloat()
except TypeError as error:
    assert '%c requires an int or a unicode character, not ' in str(error)
else:
    raise AssertionError('character formatting accepted a float protocol')
import unittest
check = unittest.TestCase()
big = 10**5001
check.assertLess(big, big + 1)
check.assertLessEqual(big, big)
check.assertGreater(big + 1, big)
check.assertGreaterEqual(big, big)
with check.assertRaises(AssertionError):
    check.assertLess(2, 1)
try:
    '%x' % 3.14
except TypeError as error:
    assert str(error) == '%x format: an integer is required, not float'
else:
    raise AssertionError('hex formatting accepted a float')
print('Python 3.14.8 reporting, APIs, argument expansion, and constructor keys agree')
