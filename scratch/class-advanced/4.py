class C:
    v = 1
    @classmethod
    def name(cls):
        return cls.__name__
    @staticmethod
    def twice(x):
        return x * 2
    def get(self):
        return self.x
    def set(self, x):
        self.x = x
    p = property(get, set)
c = C()
c.v = 2
print(C.v, c.v, c.name(), C.twice(3), c.twice(4))
del c.v
print(c.v)
c.p = 9
print(c.p)
del C.v
print(hasattr(C, "v"), callable(C), type(c) is C, c.__class__ is C)
m = c.get
print(m.__self__ is c, m.__func__ is C.get)
print(vars(C)["p"] is C.__dict__["p"])
