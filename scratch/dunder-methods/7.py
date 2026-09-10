class Key:
    def __init__(self, x):
        self.x = x
    def __hash__(self):
        print("hash", self.x)
        return self.x
    def __eq__(self, other):
        print("eq", self.x, other.x)
        return self.x == other.x
k = Key(1)
d = {k: 10, Key(1): 20}
print(d[Key(1)], k in d)
d[Key(1)] = 30
print(d[k])
del d[k]
print(len(d))
it = iter([1])
print(next(it), next(it, 9))
try:
    next(it)
except StopIteration:
    print("stopped")
