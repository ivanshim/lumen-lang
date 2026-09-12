class V:
    def __isub__(self, other):
        return "sub"
    def __imul__(self, other):
        return "mul"
    def __itruediv__(self, other):
        return "truediv"
    def __ifloordiv__(self, other):
        return "floordiv"
    def __imod__(self, other):
        return "mod"
    def __ipow__(self, other):
        return "pow"
    def __ilshift__(self, other):
        return "lshift"
    def __irshift__(self, other):
        return "rshift"
    def __iand__(self, other):
        return "and"
    def __ior__(self, other):
        return "or"
    def __ixor__(self, other):
        return "xor"
v = V()
v -= 1
print(v)
v = V()
v *= 1
print(v)
v = V()
v /= 1
print(v)
v = V()
v //= 1
print(v)
v = V()
v %= 1
print(v)
v = V()
v **= 1
print(v)
v = V()
v <<= 1
print(v)
v = V()
v >>= 1
print(v)
v = V()
v &= 1
print(v)
v = V()
v |= 1
print(v)
v = V()
v ^= 1
print(v)
class A:
    def __iadd__(self, other):
        return NotImplemented
    def __add__(self, other):
        return "fallback"
a = A()
a += 1
print(a)
