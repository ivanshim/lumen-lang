class L(list):
    def __init__(self, values):
        super().__init__(values)
        self.note = "list"
l = L([2, 1])
l.append(3)
l[0] = 4
print(l, l.note)
class S(str):
    def __new__(cls, value):
        return super().__new__(cls, value)
    def shout(self):
        return super().upper()
print(S("ab").shout())
class Q(str):
    def __eq__(self, other): return False
    def __getitem__(self, key): return "overridden"
q = Q("ab")
print(q == "ab", q[0])
