class D(dict):
    def __missing__(self, key): return 0
class L(list):
    pass
class I(int):
    def __str__(self): return "integer"
d = D(a=1)
l = L([1, 2])
l.extra = "extra"
print(d, d["z"])
print(l, l.extra)
print(I(2))
print(sorted([I(3), I(1), I(2)]))
