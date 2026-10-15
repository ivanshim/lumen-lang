class Changed(Exception):
    pass

class A:
    pass
class B(A):
    pass
class D(A):
    pass

state = {'mode': None}
class Meta(type):
    def mro(cls):
        mode = state['mode']
        if mode == 'rollback':
            assert cls.__bases__ == (B,)
            raise Changed
        if mode == 'nested':
            if cls.__bases__ == (D,):
                state['mode'] = None
            else:
                assert cls.__bases__ == (B,)
                cls.__bases__ = (D,)
                raise Changed
        return type.mro(cls)

bases = (A,)
C = Meta('C', bases, {})
original = C.__bases__
state['mode'] = 'rollback'
try:
    C.__bases__ = (B,)
except Changed:
    pass
else:
    raise AssertionError('missing rollback error')
assert C.__bases__ is original
state['mode'] = 'nested'
try:
    C.__bases__ = (B,)
except Changed:
    pass
else:
    raise AssertionError('missing outer error')
assert C.__bases__ == (D,)
assert C.__mro__ == tuple(type.mro(C))
B.__bases__ = (C,)
assert C in D.__subclasses__()
print('re-entrant bases and rollback pass')
