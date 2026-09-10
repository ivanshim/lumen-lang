def read_all():
    @support.decorate(
        'class',
    )
    class C:
        @functools.wraps(f)
        @other(a, named=b)
        def m(self):
            return 1
        @staticmethod
        def s(): return 2
        @classmethod
        def make(cls): return cls()
        @property
        def x(self): return 3
        @x.setter
        def x(self, value): pass
print('read')
