import types
import sys

value = 17

def ordinary():
    return value

attributes = {'value': 99}
ordinary.__dict__ = attributes
assert ordinary() == 17
assert ordinary.__dict__ is attributes
assert ordinary.__globals__['value'] == 17

namespace = {'value': 23, 'sys': sys}
created = types.FunctionType(ordinary.__code__, namespace)
created.__dict__ = {'value': 101}
assert created() == 23
namespace['value'] = 24
assert created() == 24
assert created.__globals__ is namespace

def generator():
    yield value
    yield sys._getframe().f_globals

made = types.FunctionType(generator.__code__, namespace)
made.__dict__ = {'value': 102}
walk = made()
assert next(walk) == 24
assert next(walk) is namespace
assert made.__globals__ is namespace
print('function attributes and live globals stay separate')
