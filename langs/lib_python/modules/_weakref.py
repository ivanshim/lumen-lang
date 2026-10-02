# The C weak-reference facade uses the kernel weak-reference handles.
from weakref import ref as _ref, proxy, getweakrefcount, getweakrefs

class ref(_ref):
    def __new__(cls, ob, callback=None):
        return object.__new__(cls)

def _remove_dead_weakref(mapping, key):
    if key in mapping and mapping[key]() is None:
        del mapping[key]
