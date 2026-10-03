# Test helpers are brought in only when asked for.
# The reference suite's own test package (CPython's Lib/test, kept at
# tests/python/test) holds data modules the embedded adapter does not,
# test.tokenizedata among them; it is added to this package's search
# path so such a module is read from there.
_here = __file__.rsplit("/", 1)[0]
_suite = _here.rsplit("/", 4)[0] + "/tests/python/test"
if _suite not in __path__:
    __path__.append(_suite)
