import warnings
class Ordinary:
    plain = 7
assert Ordinary.plain == 7
class Key:
    def __hash__(self): return hash('field')
    def __eq__(self, other): return other == 'field'
with warnings.catch_warnings(record=True):
    Mixed = type('Mixed', (), {Key(): 9})
assert Mixed.field == 9
class Child(Mixed): pass
assert Child.field == 9
class Mutable(Ordinary): pass
Mutable.__bases__ = (Mixed,)
assert Mutable.field == 9
assert Ordinary.plain == 7
print('nonstring class keys retain lookup and changed-base behavior')
