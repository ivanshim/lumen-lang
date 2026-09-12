d = {'b': 2, 'a': 1}
items = d.items()
print(len(items), ('a', 1) in items, list(items))
d['c'] = 3
for k, v in items:
    print(k, v)
print(max(d, key=d.get), sorted(d), sorted(d, reverse=True))
print(dict([((), 1), ((1, 2), 3)]), list(zip('ab', [1, 2])))
