# Imports use the same cache as import statements.
import unittest

def import_module(name, deprecated=False, required_on=None):
    try:
        return __load_module(name)
    except ImportError as msg:
        if required_on is not None:
            import sys
            if getattr(sys, 'platform', None) in required_on:
                raise
        raise unittest.SkipTest(str(msg))

# Save and restore the requested part of the import cache, with None blocking
# optional modules just as it does for an ordinary import.
def import_fresh_module(name, fresh=(), blocked=(), deprecated=False, usefrozen=False):
    import sys
    missing = object()
    requested = (name, *fresh, *blocked)
    saved = {key: sys.modules.get(key, missing) for key in requested}
    for key in requested:
        if key in sys.modules:
            del sys.modules[key]
    for key in blocked:
        sys.modules[key] = None
    try:
        module = __load_module(name)
        for key in fresh:
            __load_module(key)
        return module
    except ImportError:
        return None
    finally:
        for key in requested:
            if key in sys.modules:
                del sys.modules[key]
            if saved[key] is not missing:
                sys.modules[key] = saved[key]
