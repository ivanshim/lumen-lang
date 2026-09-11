class D:
    def __get__(self, obj, owner): return (obj is None, owner.__name__)
class C: d = D()
print(C.d, C().d)
