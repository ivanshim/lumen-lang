print("{}|{}|{}".format(2.0, ["x"], {"a": 1}))
print("{}|{}".format(1.0e20, 1.0e-7))
f = float("nan")
a = [f]
print(a.count(f), a.index(f))
a.remove(f)
print(a)
d = {"a": 1}
print(len(d.keys()), len(d.items()))
