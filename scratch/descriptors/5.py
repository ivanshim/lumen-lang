class C:
    @property
    def x(self):
        "the value"
        return self.value
    @x.setter
    def x(self, value): self.value = value
    @x.deleter
    def x(self): self.value = -1
c = C()
c.x = 5
print(c.x, C.x.__doc__, C.x.fget is C.x.fget)
del c.x
print(c.x)
def f(): return 12
s = staticmethod(f)
print(s.__func__ is f, s.__get__(None, C)())
