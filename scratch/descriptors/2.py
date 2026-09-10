class C:
    def m(self): return 1
c = C()
print(c.m == c.m, c.m is c.m, C.m is C.__dict__["m"], c.m.__self__ is c, C.m.__get__(c)())
