def replacement(self, value):
    return value + 10
def replace(f):
    return replacement
def keep(f):
    return f
class C:
    @keep
    @replace
    def m(self, value):
        return 0
    @staticmethod
    @keep
    def s(value):
        return value + 1
    @keep
    @classmethod
    def make(cls):
        return cls()
print(C().m(2))
print(C.m(C(), 3))
f = C().s
print(f(4))
print(C.make().m(5))
