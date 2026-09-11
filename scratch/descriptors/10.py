class D:
    def __get__(self, obj, owner):
        print('read', owner.__name__)
        return 11
class A:
    d = D()
    def __getattribute__(self, name):
        return super().__getattribute__(name)
    def __getattr__(self, name): return 22
a = A()
print(a.d)
print(A.d)
print(a.missing)
