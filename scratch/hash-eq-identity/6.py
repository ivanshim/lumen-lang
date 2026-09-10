class Empty:
    def __len__(self):
        return 0
class Full:
    def __bool__(self):
        return True
    def __len__(self):
        return 0
class Plain:
    pass
p = Plain()
q = Plain()
print(bool(Empty()), bool(Full()), bool(p), p is p, p is q, hash(p) == hash(p))
class Blocked:
    __hash__ = None
for value in [[], {}, set(), Blocked()]:
    try:
        hash(value)
    except TypeError as e:
        print(e)
class Left:
    def __eq__(self, other):
        return NotImplemented
class Right:
    def __eq__(self, other):
        return True
l = Left()
print(l == l, l == Left(), l != Left(), l == Right(), Right() == l)
