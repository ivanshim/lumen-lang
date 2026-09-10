s = slice(1, 5)
d = {s: "first"}
print(d[slice(1, 5)])
d[slice("x", "y")] = "second"
print(d[slice("x", "y")])
del d[slice("x", "y")]
print(len(d))
l = [1, 2]
s = slice(l)
print(s.stop is l)
l.append(3)
print(s.stop)
