# What the reference's limited C API answers about its own objects. Only
# the probes this runtime can answer truthfully stand here; the rest
# refuse at the bottom of the file.

def object_hasattrstring(obj, name):
    # Whether the object has the attribute the given bytes name spells,
    # as the limited C API's own probe answers it: 1 when it has one,
    # 0 when it has none.
    return 1 if hasattr(obj, name.decode()) else 0


def __getattr__(name):
    raise 'NotImplementedError: _testlimitedcapi.' + name + ' is not supported'
