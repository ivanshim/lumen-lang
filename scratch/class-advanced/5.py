class A:
    def __init_subclass__(cls):
        print("subclass", cls.__name__)
class B(A):
    def __class_getitem__(cls, key):
        return cls
print(B[int] is B, issubclass(B, object), isinstance(B(), (A, int)))
class C:
    def __getattribute__(self, name):
        print("get", name)
        return object.__getattribute__(self, name)
    def __delattr__(self, name):
        print("del", name)
        object.__delattr__(self, name)
c = C()
c.x = 3
print(c.x)
del c.x
print(hasattr(c, "x"))
