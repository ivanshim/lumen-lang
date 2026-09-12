d = {'a': 1, 'b': 2}
for k in d:
    d[k] += 1
print(d, len(d.popitem()), len(()), len((1,)))
f = d.get
print(f('a'), f('missing', 9))
