class T(tuple):
    def __new__(cls, value): return tuple.__new__(cls, value)
class F(float):
    pass
class D(dict):
    def __missing__(self, key): return 0
t = T([1, 2])
f = F(1.5) + 2
print(t, type(t).__name__)
print(f, type(f).__name__)
d = D(a=1)
d["z"]
print(d, sorted(D({"b": 2}).items()))
