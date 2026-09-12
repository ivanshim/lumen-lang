d = {"a": 1}
k = d.keys()
v = d.values()
i = d.items()
d["b"] = 2
print(list(k), list(v), list(i))
d["a"] = 3
print(list(v), list(i))
print(repr(k))
print(list(dict(zip("ab", [4, 5])).values()))
