import sys
class B:
    def __lshift__(self, other):
        return "left"
    def __rlshift__(self, other):
        return "rleft"
    def __rshift__(self, other):
        return "right"
    def __rrshift__(self, other):
        return "rright"
    def __and__(self, other):
        return "and"
    def __rand__(self, other):
        return "rand"
    def __or__(self, other):
        return "or"
    def __ror__(self, other):
        return "ror"
    def __xor__(self, other):
        return "xor"
    def __rxor__(self, other):
        return "rxor"
    def __imatmul__(self, other):
        return "imatmul"
    def __dir__(self):
        return ["z", "a"]
    def __sizeof__(self):
        return 20
b = B()
print(b << 1, 1 << b, b >> 1, 1 >> b)
print(b & 1, 1 & b, b | 1, 1 | b, b ^ 1, 1 ^ b)
names = dir(b)
print(names[0], names[1], sys.getsizeof(b) > 0)
b @= 2
print(b)
