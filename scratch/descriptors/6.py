def get(self): return self.absent
class C:
    p = property(fget=get, doc='kept')
    def __getattr__(self, name): return 23
c = C()
print(c.p, C.p.__doc__)
d = c.__dict__
d['x'] = 8
print(c.x)
c.x = 9
print(d['x'])
