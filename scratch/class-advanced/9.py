class A: pass
class B(A): pass
print(issubclass(B, (int, A)), isinstance(B(), (str, int, A)))
print(issubclass(int, object), issubclass(int, int), isinstance(B, type))
