import tempfile
import sys
import os
original_len, original_bytes = len, bytes
folder = tempfile.mkdtemp()
path = os.path.join(folder, 'wildcard_fixture.py')
with open(path, 'w') as stream:
    stream.write("__all__ = ['helper', 'plain']\ndef helper(value): return value + 1\nplain = 7\n")
sys.path.insert(0, folder)
from wildcard_fixture import *
assert helper(plain) == 8
assert len([1, 2]) == 2
assert bytes([65]) == b'A'
import wildcard_fixture
wildcard_fixture.__all__ = ['helper', 'len', 'bytes']
wildcard_fixture.len = lambda value: 23
wildcard_fixture.bytes = lambda value: 'changed'
from wildcard_fixture import *
assert helper(plain) == 8
assert len([]) == 23
assert bytes([]) == 'changed'
globals()['len'] = lambda value: 31
assert len([]) == 31
globals()['len'] = original_len
globals()['bytes'] = original_bytes
assert len([1, 2, 3]) == 3
assert bytes([66]) == b'B'
sys.path.remove(folder)
os.unlink(path)
os.rmdir(folder)
print('wildcard helpers and native overrides retain live shared bindings')
