# Runtime adapter for CPython v3.14.8 functools.
# Retains the documented native-object bridges previously mixed into the source.
# The upstream module is initialized first, then these bindings supply runtime protocols.

def total_ordering(cls):
    """Class decorator that fills in missing ordering methods"""
    # Find user-defined comparisons (not those inherited from object).
    roots = {op for op in _convert if any(op in base.__dict__ for base in cls.__mro__ if base is not object)}
    if not roots:
        raise ValueError('must define at least one ordering operation: < > <= >=')
    root = max(roots)       # prefer __lt__ to __le__ to __gt__ to __ge__
    for opname, opfunc in _convert[root]:
        if opname not in roots:
            opfunc.__name__ = opname
            setattr(cls, opname, opfunc)
    return cls

class CacheInfo(tuple):
    __slots__ = ()
    _fields = ('hits', 'misses', 'maxsize', 'currsize')
    def __iter__(self):
        return iter((self[0], self[1], self[2], self[3]))
    def __new__(cls, hits, misses, maxsize, currsize):
        return tuple.__new__(cls, (hits, misses, maxsize, currsize))
    @property
    def hits(self):
        return self[0]
    @property
    def misses(self):
        return self[1]
    @property
    def maxsize(self):
        return self[2]
    @property
    def currsize(self):
        return self[3]
    def __repr__(self):
        return 'CacheInfo(hits=%r, misses=%r, maxsize=%r, currsize=%r)' % tuple(self)

_CacheInfo = CacheInfo

del CacheInfo

_CacheInfo.__module__ = __name__

for _class in (partial, partialmethod, _PlaceholderType, singledispatchmethod,
               _singledispatchmethod_get, cached_property):
    _class.__module__ = __name__

del _class


# Native partial formatting retains slot references before user representations.
def _partial_repr(self, _base=partial):
    return __partial_repr__(self, _base)

partial.__repr__ = recursive_repr()(_partial_repr)
