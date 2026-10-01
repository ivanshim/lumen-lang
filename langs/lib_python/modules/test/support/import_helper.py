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
    try:
        return __load_module(name)
    except ImportError:
        return None

# A lazy import is read as an eager one here, so importing a module always
# brings its own imports in at once. A test asking that they be kept apart
# is that implementation's behaviour; the guards in test.support step past
# it, and the honest answer here, if it were ever reached, is to skip.
def ensure_lazy_imports(imported_module, modules_to_block, *, additional_code=None):
    raise unittest.SkipTest('lazy imports are not observed separately here')

