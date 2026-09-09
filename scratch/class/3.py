class P:
    def __init__(self, x):
        self.x = x
    def get(self):
        return self.x

class Q(P):
    def get(self):
        return super().get() + 1

xs = []
xs.append(3)
q = Q(4)
print(q.get(), len(xs), xs)
