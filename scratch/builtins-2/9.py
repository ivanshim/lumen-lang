g = {'a': 2}
l = {'a': 5}
print(eval('a + 1', g, l))
exec("b = a + 2; locals()['a'] = 9; print(a, b)", g, l)
print(g['a'], l['a'], l['b'])
exec('global a; a = 7; c = 8', g, l)
print(g['a'], l['a'], l['c'])
exec('x = 3', g, g)
print(g['x'])
exec("del b; print('b' in locals())", g, l)
