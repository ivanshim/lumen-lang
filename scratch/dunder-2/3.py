class A:
    def __add__(self, other):
        return NotImplemented
class B:
    def __radd__(self, other):
        return NotImplemented
print(A() + B())
