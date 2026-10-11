import builtins
import tempfile
import os
import sys
original = builtins.len
directory = tempfile.mkdtemp()
path = os.path.join(directory, 'override_fixture.py')
with open(path, 'w') as stream:
    stream.write('def read(value): return len(value)\n')
sys.path.insert(0, directory)
import override_fixture as module
assert module.read([1, 2]) == 2
builtins.__dict__['len'] = lambda value: 17
assert module.read([]) == 17
builtins.__dict__['len'] = original
assert module.read([1]) == 1
module.__dict__['len'] = lambda value: 23
assert module.read([]) == 23
del module.__dict__['len']
assert module.read([1, 2, 3]) == 3
module.__dict__['__all__'] = ['len']
module.__dict__['len'] = lambda value: 31
exec('from override_fixture import *\nassert len([]) == 31')
assert module.read([]) == 31
globals()['len'] = lambda value: 41
assert len([]) == 41
del globals()['len']
assert len([1, 2]) == 2
sys.path.remove(directory)
os.unlink(path)
os.rmdir(directory)
print('namespace and builtin replacements remain live across cache reads')
