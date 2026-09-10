class Places:
    def __getitem__(self, key):
        return [0, 1, 2, 3, 4][key]
    def __setitem__(self, key, value):
        print("set", [0, 1, 2, 3, 4][key], value)
    def __delitem__(self, key):
        print("del", [0, 1, 2, 3, 4][key])
p = Places()
print(p[1:4:2])
p[1:4:2] = [7, 8]
del p[:3]
class N:
    def __eq__(self, other):
        return True
print(N() != N())
class R:
    def __sub__(self, other):
        return 10
    def __rsub__(self, other):
        return 11
    def __truediv__(self, other):
        return 12
    def __rtruediv__(self, other):
        return 13
    def __floordiv__(self, other):
        return 14
    def __rfloordiv__(self, other):
        return 15
    def __mod__(self, other):
        return 16
    def __rmod__(self, other):
        return 17
    def __pow__(self, other):
        return 18
    def __rpow__(self, other):
        return 19
    def __neg__(self):
        return 20
r = R()
print(r - 1, 1 - r, r / 1, 1 / r, r // 1, 1 // r)
print(r % 1, 1 % r, r ** 1, 1 ** r, -r)
