# What the reference's limited C API answers about its own objects. Only
# the probes this runtime can answer truthfully stand here; the rest
# refuse at the bottom of the file.

def object_hasattrstring(obj, name):
    # Whether the object has the attribute the given bytes name spells,
    # as the limited C API's own probe answers it: 1 when it has one,
    # 0 when it has none.
    return 1 if hasattr(obj, name.decode()) else 0


def sequence_getitem(obj, i):
    # The raw one-place sequence read. For a kind whose own one-place
    # read is its subscript -- bytes and bytearray both -- the two are
    # the very same working, so the subscript is the honest answer.
    return obj[i]


def sequence_setitem(obj, i, value):
    # The raw one-place sequence write, the subscript again for these
    # kinds.
    obj[i] = value


def sequence_delitem(obj, i):
    # The raw sequence place given up, the subscript again.
    del obj[i]


def __getattr__(name):
    raise 'NotImplementedError: _testlimitedcapi.' + name + ' is not supported'
