# What the reference's limited C API answers about its own objects. Only
# the probes this runtime can answer truthfully stand here; the rest
# refuse at the bottom of the file.

def object_hasattrstring(obj, name):
    # Whether the object has the attribute the given bytes name spells,
    # as the limited C API's own probe answers it: 1 when it has one,
    # 0 when it has none.
    return 1 if hasattr(obj, name.decode()) else 0


def _sequence(obj):
    # Native sequence slots are supported; other kinds are truthfully refused.
    if isinstance(obj, dict):
        raise TypeError("mapping has no sequence slot")
    if not isinstance(obj, (bytes, bytearray, str, list, tuple, range)):
        raise NotImplementedError("sequence slots for this receiver are unsupported")


def sequence_getitem(obj, i):
    # Read through a supported native sequence slot.
    _sequence(obj)
    return obj[i]


def sequence_setitem(obj, i, value):
    # Write through a supported native sequence slot.
    _sequence(obj)
    obj[i] = value


def sequence_delitem(obj, i):
    # Delete through a supported native sequence slot.
    _sequence(obj)
    del obj[i]


def __getattr__(name):
    raise 'NotImplementedError: _testlimitedcapi.' + name + ' is not supported'
