# Runtime adapter for CPython v3.14.8 copyreg.
# Retains the documented native-object bridges previously mixed into the source.
# The upstream module is initialized first, then these bindings supply runtime protocols.

_new_type = type(len)

