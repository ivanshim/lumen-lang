import sys
import types
lookup = __load_module
name = '_cache_probe_module'
assert name not in sys.modules
assert lookup(name, 'cached') is None
assert name not in sys.modules
module = types.ModuleType(name)
sys.modules[name] = module
assert lookup(name, 'cached') is module
sys.modules[name] = None
assert lookup(name, 'cached') is None
del sys.modules[name]
assert lookup(name, 'cached') is None
for index in range(80):
    sys.modules[name] = module
    assert lookup(name, 'cached') is module
    del sys.modules[name]
    assert lookup(name, 'cached') is None
original = sys.modules
try:
    sys.modules = dict(original)
    sys.modules[name] = module
    assert lookup(name, 'cached') is module
    del sys.modules[name]
    assert lookup(name, 'cached') is None
finally:
    sys.modules = original
assert lookup('sys', 'cached') is sys
print('cache-only module lookup preserves live replacement and deletion')
