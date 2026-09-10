class P:
    def __init__(self, x):
        self.x = x
    def get(self):
        return self.x

class Q(P):
    def get(self):
        return super().get() + 1

print(Q(4).get())
