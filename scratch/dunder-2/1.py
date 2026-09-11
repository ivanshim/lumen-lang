class U:
    def __neg__(self):
        return "neg"
    def __pos__(self):
        return "pos"
    def __abs__(self):
        return "abs"
    def __invert__(self):
        return "invert"
    def __matmul__(self, other):
        return "matmul"
    def __rmatmul__(self, other):
        return "rmatmul"
u = U()
print(-u, +u, abs(u), ~u)
print(u @ 2, 2 @ u)
class Counter:
    def __init__(self):
        self.n = 1
    def __iadd__(self, value):
        self.n = self.n + value
        return self
c = Counter()
alias = c
c += 2
print(c.n, alias.n)
class Add:
    def __add__(self, value):
        return value + 10
a = Add()
a += 3
print(a)
