class Pair:
    pass
class P:
    def __init__(self):
        self.pair = Pair()
        self.pair.left: int = 1
print(P().pair.left)
