class P(property):
    def __get__(self, obj, owner):
        if obj is None: return self
        return super().__get__(obj, owner) + 10
class C:
    @P
    def x(self): return 7
    def __getattr__(self, name):
        print('fallback', name)
        return 99
def f(cls): return cls.__name__
print(C().x)
print(classmethod(f).__get__(None, C)())
