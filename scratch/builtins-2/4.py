x = 3
g = globals()
x = 8
print(g['x'])
g['x'] = 10
print(x)
print(g is globals())
ns = {'a': 6}
exec('def twice():\n    return a * 2', ns)
print(ns['twice']())
ns['a'] = 7
print(ns['twice']())
print('twice' in globals(), '__builtins__' in ns)
x += 1
print(x, g['x'])
del x
print('x' in g)
