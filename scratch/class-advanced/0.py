class A: pass
class B(A): pass
print(B.__mro__, issubclass(B, (A, int)), B.__name__, isinstance(B(), object))
