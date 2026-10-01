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

def _save_and_remove_modules(names):
    import sys
    saved = {}
    for module_name in list(sys.modules):
        selected = module_name in names
        for name in names:
            if module_name.startswith(name + '.'):
                selected = True
        if selected:
            saved[module_name] = sys.modules[module_name]
            del sys.modules[module_name]
    return saved


def import_fresh_module(name, fresh=(), blocked=(), deprecated=False, usefrozen=False):
    """Import an isolated copy, restoring the original cache afterwards.

    Missing required fresh dependencies return None. Blocked names occupy
    sys.modules with None, making imports of them raise ImportError.
    This interpreter has no frozen modules or deprecated import machinery.
    """
    import sys
    fresh = list(fresh)
    blocked = list(blocked)
    names = [name] + fresh + blocked
    saved = _save_and_remove_modules(names)
    for module_name in blocked:
        sys.modules[module_name] = None
    try:
        try:
            for module_name in fresh:
                __load_module(module_name)
        except ImportError:
            return None
        return __load_module(name)
    finally:
        _save_and_remove_modules(names)
        for module_name in saved:
            sys.modules[module_name] = saved[module_name]
