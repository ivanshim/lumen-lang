class S:
    __slots__ = ('x',)
class T:
    __slots__ = ('x',)
t = T()
t.x = 9
S.x.__get__(t)
