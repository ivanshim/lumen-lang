class D:
    def __get__(self, obj, owner): return obj.value
    def __set__(self, obj, value): obj.value = value
    def __delete__(self, obj): obj.value = -1
class C:
    x = D()
    def __getattribute__(self, name): return object.__getattribute__(self, name)
c = C()
c.x = 4
print(c.x)
del c.x
print(c.x)
class S:
    __slots__ = ('a',)
s = S()
S.a.__set__(s, 8)
print(S.a.__get__(s, S), s.a)
