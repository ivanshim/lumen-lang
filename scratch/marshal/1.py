import marshal
import types
import io
import os

# Different instruction bodies and constants remain different keys.
for part in compile("def f(a, b): return a + b", "same.py", "exec").co_consts:
    if isinstance(part, types.CodeType):
        one = part
for part in compile("def f(a, b): return a - b", "same.py", "exec").co_consts:
    if isinstance(part, types.CodeType):
        two = part
print("bodies", one == two, len({one: 1, two: 2}))

# Restore appended constants, flags, sharing, and nested replacement bodies.
def base():
    return 1000
code = base.__code__
shared = [1, 2]
changed = code.replace(co_consts=code.co_consts + (shared, shared), co_flags=code.co_flags | 64)
loaded = marshal.loads(marshal.dumps(changed))
print("state", loaded.co_consts == changed.co_consts, loaded.co_flags == changed.co_flags, loaded.co_consts[-1] is loaded.co_consts[-2])
# Hashable code objects with distinct literal tables must occupy distinct keys.
key = code.replace(co_consts=code.co_consts + (1001,))
values = {code: "base", key: "changed"}
print("keys", len(values), values[code], values[key], values[marshal.loads(marshal.dumps(code))])

def outer():
    def inner(a, b):
        return a + b
    return inner
parent = outer.__code__
parts = list(parent.co_consts)
for at, part in enumerate(parts):
    if isinstance(part, types.CodeType):
        parts[at] = two
replaced = parent.replace(co_consts=tuple(parts))
restored = marshal.loads(marshal.dumps(replaced))
print("nested", restored == replaced)
print("nested call", types.FunctionType(restored, {})()(7, 2))

# A live routine survives edits and deletion of the file it came from.
filename = "@test_marshal_live.py"
with open(filename, "w") as file:
    file.write("def live(a, b): return a + b\n")
with open(filename) as file:
    block = compile(file.read(), filename, "exec")
for part in block.co_consts:
    if isinstance(part, types.CodeType):
        live = part
with open(filename, "w") as file:
    file.write("def live(a, b): return a - b\n")
first = marshal.loads(marshal.dumps(live))
os.unlink(filename)
second = marshal.loads(marshal.dumps(live))
print("file lifetime", types.FunctionType(first, {})(7, 2), types.FunctionType(second, {})(7, 2))

# No positioning method is needed by a binary readinto stream.
class Reader:
    def __init__(self, data):
        self.file = io.BytesIO(data)
    def read(self, n):
        return self.file.read(n)
    def readinto(self, target):
        return self.file.readinto(target)
class Nonseekable(Reader):
    def tell(self):
        raise io.UnsupportedOperation("not seekable")
for kind in (Reader, Nonseekable):
    reader = kind(marshal.dumps(1234) + marshal.dumps("next"))
    print("stream", marshal.load(reader), marshal.load(reader))
for direction in ("write", "read"):
    try:
        if direction == "write":
            marshal.dumps(code, allow_code=False)
        else:
            marshal.loads(marshal.dumps(code), allow_code=False)
    except ValueError:
        print("allow_code", direction, "rejected")

import marshal
import types
import __future__
source = 'def f():\n    "doc"\n    assert True\n    return (1000, __debug__)\n'
for level in range(3):
    root = compile(source, 'options.py', 'exec', optimize=level)
    for value in root.co_consts:
        if isinstance(value, types.CodeType):
            code = value
    loaded = marshal.loads(marshal.dumps(code))
    fn = types.FunctionType(loaded, {})
    print('optimize', level, loaded == code, loaded.co_consts == code.co_consts, fn(), fn.__doc__)
root = compile('def f(x: Missing): return x\n', 'future.py', 'exec', flags=__future__.annotations.compiler_flag, dont_inherit=True)
for value in root.co_consts:
    if isinstance(value, types.CodeType) and value.co_name == 'f':
        code = value
loaded = marshal.loads(marshal.dumps(code))
print('future', code == loaded, types.FunctionType(loaded, {})(7))

import marshal
import types

def f():
    return 1000
line = marshal.loads(marshal.dumps(1000))
flags = marshal.loads(marshal.dumps(f.__code__.co_flags | 1024))
changed = f.__code__.replace(co_firstlineno=line, co_flags=flags)
loaded = marshal.loads(marshal.dumps(changed))
print('boxed metadata', loaded.co_firstlineno == line, loaded.co_flags == flags)
parts = tuple(2000 if value == 1000 else value for value in f.__code__.co_consts)
changed = f.__code__.replace(co_consts=parts)
print('changed literal', types.FunctionType(marshal.loads(marshal.dumps(changed)), {})())
try:
    marshal.dumps(f.__code__.replace(co_consts=f.__code__.co_consts + (bytes,)))
except ValueError:
    print('user type', 'rejected')

import marshal
import __future__
root = compile('def f(x: Missing): return x\n', 'root.py', 'exec', flags=__future__.annotations.compiler_flag, dont_inherit=True)
changed = root.replace(co_flags=0)
loaded = marshal.loads(marshal.dumps(changed))
namespace = {}
exec(loaded, namespace)
print('root flags', loaded.co_flags == 0, namespace['f'](7), namespace['f'].__annotations__)

import marshal
source = "class A:\n    def first(self): return 1\n    def last(self): return 2\n"
code = compile(source, 'class.py', 'exec', dont_inherit=True)
loaded = marshal.loads(marshal.dumps(code))
namespace = {}
exec(loaded, namespace)
a = namespace['A']()
print('class body', code == loaded, a.first(), a.last())

import marshal
import types
def f():
    return 1000
code = f.__code__.replace(co_linetable=b'')
loaded = marshal.loads(marshal.dumps(code))
print('line state', code == loaded, hash(code) == hash(loaded), types.FunctionType(loaded, {})())

import marshal
import types
outer_source = 'def outer():\n    def inner(v): return v + OLD\n    return inner(4)\n'
inner_source = 'def alternate(v): return v * NEW\n'
for value in compile(outer_source, 'outer.py', 'exec').co_consts:
    if isinstance(value, types.CodeType):
        outer = value
for value in compile(inner_source, 'inner.py', 'exec').co_consts:
    if isinstance(value, types.CodeType):
        inner = value
parts = tuple(inner if isinstance(value, types.CodeType) else value for value in outer.co_consts)
changed = outer.replace(co_consts=parts)
loaded = marshal.loads(marshal.dumps(changed))
print('nested globals', loaded == changed, types.FunctionType(loaded, {'NEW': 3, 'OLD': 100})())

import marshal
import types
namespace = {'NEW': 8}
def make():
    def inner(v):
        return v if v else NEW
    return inner
fn = types.FunctionType(marshal.loads(marshal.dumps(make.__code__)), namespace)()
print('nested namespace', fn(0), fn(4), fn.__globals__ is namespace)
def capture(x):
    def inner(v):
        return x + v + NEW
    return inner
fn = types.FunctionType(marshal.loads(marshal.dumps(capture.__code__)), namespace)(5)
print('captured namespace', fn(2), fn.__globals__ is namespace)
def writer():
    def inner(v):
        global NEW
        NEW = v
        return NEW
    return inner
fn = types.FunctionType(marshal.loads(marshal.dumps(writer.__code__)), namespace)()
print('namespace write', fn(12), namespace['NEW'])

import marshal
import types
def deleter():
    def inner():
        global DATA
        del DATA
        try:
            return DATA
        except NameError:
            return 'missing'
    return inner
namespace = {'DATA': 8}
f = types.FunctionType(marshal.loads(marshal.dumps(deleter.__code__)), namespace)()
print('namespace deletion', f(), 'DATA' in namespace)

import marshal
code = compile('class A: pass', 'empty.py', 'exec')
namespace = {}
exec(marshal.loads(marshal.dumps(code)), namespace)
print('class module', namespace['A'].__module__)

import marshal
import types
SECRET = 99
def read():
    return SECRET
for code in (read.__code__, marshal.loads(marshal.dumps(read.__code__))):
    try:
        print('isolated', types.FunctionType(code, {})())
    except NameError:
        print('isolated', 'missing')

gns = {}
lns = {}
exec('x: int = 5\n', gns, lns)
gns.update(lns)
print('annotation namespace', lns['__annotate__'](1) == {'x': int})

import marshal
import types
def outer(x):
    def inner(y):
        return x + y
    return inner
fn = outer(3)
code = marshal.loads(marshal.dumps(fn.__code__))
restored = types.FunctionType(code, {}, closure=fn.__closure__)
print('closure code', code == fn.__code__, restored(4))
def generate(n):
    for i in range(n):
        yield i + 1000
code = marshal.loads(marshal.dumps(generate.__code__))
print('generator code', code == generate.__code__, list(types.FunctionType(code, {})(3)))

import types
source = 'def f(x):\n    assert x\n    return None\n'
def take(root):
    for value in root.co_consts:
        if isinstance(value, types.CodeType) and value.co_name == 'f':
            return value
one = take(compile(source, 'opt.py', 'exec', optimize=0))
two = take(compile(source, 'opt.py', 'exec', optimize=1))
print('assert body', one == two, len({one: 1, two: 2}))
source = 'def f():\n    return X\n'
one = take(compile(source, 'same.py', 'exec'))
root = compile('def outer(X):\n    def f():\n        return X\n    return f\n', 'same.py', 'exec')
for outer in root.co_consts:
    if isinstance(outer, types.CodeType) and outer.co_name == 'outer':
        two = take(outer).replace(co_firstlineno=one.co_firstlineno, co_flags=one.co_flags)
print('binding body', one == two, len({one: 1, two: 2}))

source = 'def f(x):\n    return x\n'
one = take(compile(source, 'plain.py', 'exec', optimize=0))
two = take(compile(source, 'plain.py', 'exec', optimize=1))
print('unchanged body', one == two, hash(one) == hash(two), len({one: 1, two: 2}))

from collections import namedtuple
from copy import copy
Point = namedtuple('Point', 'x y')
point = Point(4, 7)
print('copied tuple', tuple(copy(point)))
print('tuple newargs', tuple(Point(*point.__getnewargs__())))
