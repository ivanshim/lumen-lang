class Base: pass
class Child(Base):
    @property
    def __class__(self):
        raise AssertionError('real subtype must precede reported class')
assert isinstance(Child(), Base)
class Spoof:
    @property
    def __class__(self): return Base
assert isinstance(Spoof(), Base)
class Meta(type):
    def __instancecheck__(cls, value): return False
class HookBase(metaclass=Meta): pass
class HookChild(HookBase): pass
assert not isinstance(HookChild(), HookBase)
assert isinstance(HookChild(), HookChild)
assert not isinstance(object(), Base)
print('ok')
