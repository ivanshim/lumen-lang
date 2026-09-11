d = {"a": 1}
it = iter(d.keys())
d["b"] = 2
next(it)
