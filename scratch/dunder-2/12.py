class I:
    def __index__(self):
        return 2
v = [10, 20, 30, 40]
v[I()] = 31
print(v)
print(list(range(I())), v[:I()], v[::I()])
del v[I()]
print(v)
class P:
    def __rpow__(self, base, modulus=None):
        return 6
print(pow(3, P(), 7))
