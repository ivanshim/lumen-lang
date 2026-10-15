import gc
source = 'def outer():\n'
for i in range(130):
    source += '    v' + str(i) + ' = ' + str(i) + '\n'
source += '    def read():\n        return (' + ','.join('v' + str(i) for i in range(130)) + ')\n'
source += '    def change():\n        nonlocal v0, v64, v129\n        v0 += 1000\n        v64 += 1000\n        v129 += 1000\n'
source += '    return read, change\n'
scope = {}
exec(source, scope)
read, change = scope['outer']()
expected = list(range(130))
assert read() == tuple(expected)
change()
for i in (0, 64, 129):
    expected[i] += 1000
assert read() == tuple(expected)
gc.collect()
assert read() == tuple(expected)
change()
for i in (0, 64, 129):
    expected[i] += 1000
assert read() == tuple(expected)
print('closure slots preserve wide captures and shared writes')
