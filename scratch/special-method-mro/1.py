class Base:
    def __len__(self): return 2
class Other:
    def __len__(self): return 3
class Meta(type):
    def mro(cls): return [cls, Other, Base, object]
class Custom(Base, metaclass=Meta): pass
assert len(Custom()) == 3
class Mixin:
    def __len__(self): return 9
class NativeFirst(list, Mixin): pass
class MixinFirst(Mixin, list): pass
assert len(NativeFirst([1, 2])) == 2
assert len(MixinFirst([1, 2])) == 9
class NoHash:
    def __eq__(self, other): return self is other
try: hash(NoHash())
except TypeError: pass
else: raise AssertionError('equality-only class must be unhashable')
print('special methods retain custom MRO and native slot precedence')
