pairs = {'a': 1, 'b': 2}
merged = {**pairs, 'a': 3, **{'c': 4}}
print(len(merged), merged['a'], merged['b'], merged['c'])
print(len({0, *[1, 2], *{3: 9}}))
print({k: {j: k + j for j in range(3)} for k in range(2)}[1][2])
