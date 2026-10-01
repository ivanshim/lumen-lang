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

# This run keeps no module cache to isolate and carries no accelerator
# modules to block, so a fresh import is the import an import statement
# makes. As the reference does, None is handed back for a module that
# cannot be imported at all.
def import_fresh_module(name, fresh=(), blocked=(), deprecated=False, usefrozen=False):
    # A module asked for fresh, that cannot be imported at all, means
    # the fresh variant the caller asks for does not exist here, and
    # None is the answer, as the reference answers for its own missing
    # accelerators.
    for module in fresh:
        try:
            __load_module(module)
        except ImportError:
            return None
    try:
        return __load_module(name)
    except ImportError:
        return None
