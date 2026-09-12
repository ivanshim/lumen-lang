d = {"a": 1}
k = d.keys()
v = d.values()
print(len(k), len(v), "a" in k, 1 in v)
d["b"] = 2
print(list(k), list(v), k, v)
for value in v:
    print(value)
d.clear()
print(len(k), len(v))
