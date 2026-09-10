class A:
    __slots__ = ('x',)
class B(A):
    __slots__ = ('x',)
b = B()
A.x.__set__(b, 1)
B.x.__set__(b, 2)
print(A.x.__get__(b), b.x)
del b.x
print(A.x.__get__(b))
