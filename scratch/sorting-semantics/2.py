class L:
    def __init__(self, value):
        self.value = value
    def __lt__(self, other):
        return self.value < other.value
print([v.value for v in sorted([L(3), L(1), L(2)])])
print(min([], default="none"), max(3, 1, 2), max([2, 2.0], key=lambda v: 0))
l = [1, 2]
print(l.sort())
