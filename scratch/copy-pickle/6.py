import pickle
class Item:
    def __init__(self, value):
        self.value = value
    def __getstate__(self):
        return self.value + 1
    def __setstate__(self, state):
        self.value = state - 1
print(pickle.loads(pickle.dumps(Item(7))).value)
class Link:
    pass
a = Link()
a.child = a
b = pickle.loads(pickle.dumps(a))
print(b.child is b, b is a)
