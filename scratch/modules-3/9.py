#!/usr/bin/env python3
import types
import builtins
# Check live module dictionaries, builtin values, and nested text contexts.
for name in ('alpha', 'beta'):
    module = types.ModuleType(name)
    namespace = module.__dict__
    exec('''
import types
seed = 23
seen = globals()
assert bool.__new__(bool) is False
assert callable(id)
assert types.MethodType(id, object()).__func__ is id
assert sorted([-9, 3, -1], key=abs) == [-1, 3, -9]
def read(local=4):
    return eval('seed + local')
def rewrite():
    global seed
    seed = 41
    return eval('seed')
def remove():
    global seed
    del seed
    return 'seed' not in globals()
''', namespace)
    assert namespace is module.__dict__
    assert namespace['seen'] is namespace
    assert module.read() == 27
    assert module.rewrite() == 41
    assert module.seed == 41
    assert module.remove() is True
    assert 'seed' not in namespace
    namespace['seed'] = 57
    assert module.read() == 61
    del namespace['seed']
    assert not hasattr(module, 'seed')
    print(name, 'namespace ok')
# Keep explicit builtins and local precedence intact.
space = {'__builtins__': {'abs': lambda n: 88}}
exec('result = abs(-7)', space)
assert space['result'] == 88
outer = {'value': 10}
inner = {'value': 19}
assert eval('value', outer, inner) == 19
# Read defining globals even when invoked from a different module namespace.
source = {'value': 37}
exec("def lookup():\n    local = 6\n    return eval('value + local')", source)
other = {'value': -1, 'lookup': source['lookup']}
exec('result = lookup()', other)
assert other['result'] == 43
print('contexts ok')
# Preserve aliases when a builtin is rebound after the default dictionary exists.
original = builtins.abs
builtins.abs = lambda value: 999
try:
    namespace = {}
    exec('answer = abs(-3)', namespace)
    assert namespace['answer'] == 999
finally:
    builtins.abs = original
# Module-level nested text writes into the same live namespace.
namespace = {}
exec("exec('value = 6')", namespace, namespace)
assert namespace['value'] == 6
print('live bindings ok')
