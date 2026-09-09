class C:
    marker = 1
    @classmethod
    def mark(cls):
        return cls.marker
    @property
    def x(self):
        return self._x
    @x.setter
    def x(self, value):
        self._x = value + 1
class D(C):
    marker = 2
d = D()
d.x = 5
print(d.x)
print(D.mark())
print(d.mark())
f = d.mark
print(f())
