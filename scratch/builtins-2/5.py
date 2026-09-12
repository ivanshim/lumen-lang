ns = {'n': 4}
exec('q = n + 1', ns)
print(ns['q'])
print(eval('n * 3', ns))
c = compile(source='n + 2', filename='<s>', mode='eval', flags=0, dont_inherit=True)
print(eval(c, ns))
e = eval
print(e('2 + 4'))
print(__name__, __doc__, '__file__' in globals(), '__builtins__' in globals())
print(hash(-1), hash(True), hash(123))
def f():
    v = 9
    print('v' in vars(), 'v' in dir())
f()
a = []
b = a
c = []
print(id(a) == id(b), id(a) == id(c), id(None) == id(None))
print(id(1) == id(1), id(True) == id(1))
print(hash(1.0) == hash(1), hash(2 ** 70), hash(-1.0))
