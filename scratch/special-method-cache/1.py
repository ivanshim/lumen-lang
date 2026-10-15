class A:
    def __len__(self): return 1
class B(A): pass
b = B()
assert len(b) == 1
A.__len__ = lambda self: 2
assert len(b) == 2
class C:
    def __len__(self): return 3
B.__bases__ = (C,)
assert len(b) == 3
print('special method caches follow mutation and changed bases')
