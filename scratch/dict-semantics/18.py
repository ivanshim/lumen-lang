d = {'a': 1}
keys = reversed(d)
d['b'] = 2
for k in keys:
    del d[k]
