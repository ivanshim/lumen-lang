class V:
    def __init__(self, x):
        self.x = x
    def __add__(self, other):
        return self.x + other
    def __radd__(self, other):
        return other + self.x
    def __mul__(self, other):
        return self.x * other
    def __call__(self, other):
        return self.x + other
    def __bool__(self):
        return self.x != 0
    def __hash__(self):
        return self.x
    def __eq__(self, other):
        return self.x == other.x
v = V(3)
print(v + 4, 4 + v, v * 4, v(4))
print(bool(v), bool(V(0)), hash(v))
print(len({v, V(3)}))
print({v: "yes"}[V(3)])
