class U(set):
    pass
u = U([2, 1, 2])
u.note = "set"
print(sorted(u), len(u), 1 in u, u.note, isinstance(u, set), type(u) is U)
class I(int):
    pass
i = I(3)
print(i + 2, type(i + 2) is int, {i: "three"}[3], hash(i) == hash(3))
class L(list):
    def __new__(cls, values):
        return list.__new__(cls, values)
l = L([1, 2])
print(l, hasattr(l, "__dict__"), isinstance(l, object))
class O(object):
    pass
print(isinstance(O(), object))
