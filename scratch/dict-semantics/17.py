a = {'a': 1, 'b': 2}
b = {'b': 2, 'c': 3}
print(sorted(a.keys() & b.keys()))
print(sorted(a.keys() | b.keys()))
print(list(a.items() & b.items()))
print(len(a.items() | b.items()), ('a', 1) in (a.items() | b.items()))
print(a.keys() & {}.keys())
