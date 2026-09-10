class Key:
    def __hash__(self):
        return 7
    def __eq__(self, other):
        return isinstance(other, Key)
k = Key()
d = {k: 1, Key(): 2}
print(len(d), d[k], d[Key()], len(set([k, Key()])))
d[Key()] = 3
print(len(d), d[k], dict([(k, 4)])[Key()])
class Unhashable:
    def __eq__(self, other):
        return True
try:
    d = {Unhashable(): 1}
except TypeError as e:
    print(e)
