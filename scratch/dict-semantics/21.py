d = dict(a=1, b=2)
e = {'b': 4, 'c': 3}
print({**d, **e}, d != e)
print({k: v for k, v in d.items() if v > 1})
print(dict(**d), dict(d, b=5))
print({True: 'first', 1.0: 'last'}, {(1, True): 3}[(1.0, 1)])
