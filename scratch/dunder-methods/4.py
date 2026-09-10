class A:
    tag = 7
class B(A):
    pass
b = B()
print(b, str(b))
print(b == b, b == B())
print(isinstance(b, A), isinstance(b, B), b.__class__.__name__)
print(b.tag)
b.tag = 9
print(b.tag, B.tag, A.tag, b.__dict__["tag"])
class Box:
    def __init__(self):
        self.items = [10, 20, 30]
    def __len__(self):
        return len(self.items)
    def __getitem__(self, key):
        return self.items[key]
    def __setitem__(self, key, value):
        self.items[key] = value
    def __delitem__(self, key):
        del self.items[key]
x = Box()
print(x[1:], bool(x))
x[1] = 40
del x[0]
print(x[:], len(x))
