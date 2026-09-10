class C:
    @classmethod
    def make(cls):
        return cls()
    @property
    def x(self):
        return self._x
    @x.setter
    def x(self, value):
        self._x = value
c = C.make()
c.x = 7
print(c.x)
d = c.make()
d.x = 9
print(d.x)
print(c.x)
