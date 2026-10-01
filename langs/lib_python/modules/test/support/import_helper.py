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

# Requested dependencies must exist before the target is imported. As in
# CPython, an unavailable accelerator makes a fresh import return None.
# Blocking an absent accelerator needs no change to the module cache.
def import_fresh_module(name, fresh=(), blocked=(), deprecated=False, usefrozen=False):
    try:
        for dependency in fresh:
            __load_module(dependency)
        return __load_module(name)
    except ImportError:
        return None
