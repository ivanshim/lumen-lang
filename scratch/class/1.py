class P:
    count = 0
    def __init__(self, x):
        self.x = x
    def get(self):
        return self.x

a = P(3)
b = P(7)
print(a.get())
print(b.x)
print(P.count)
