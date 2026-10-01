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
        for dependency in fresh:
            __load_module(dependency)
        return __load_module(name)
    except ImportError:
        return None
    finally:
        for key in requested:
            if key in sys.modules:
                del sys.modules[key]
            if saved[key] is not missing:
                sys.modules[key] = saved[key]


# From CPython 3b564385e4c9, Lib/test/support/import_helper.py; PSF License.
import textwrap

def ensure_lazy_imports(imported_module, modules_to_block, *, additional_code=None):
    """Test that when imported_module is imported, none of the modules in
    modules_to_block are imported as a side effect."""
    modules_to_block = frozenset(modules_to_block)
    script = textwrap.dedent(
        f"""
        import sys
        modules_to_block = {modules_to_block}
        if unexpected := modules_to_block & sys.modules.keys():
            startup = ", ".join(unexpected)
            raise AssertionError(f'unexpectedly imported at startup: {{startup}}')

        import {imported_module}
        if unexpected := modules_to_block & sys.modules.keys():
            after = ", ".join(unexpected)
            raise AssertionError(f'unexpectedly imported after importing {imported_module}: {{after}}')
        """
    )
    if additional_code:
        script += additional_code
        script += textwrap.dedent(
            f"""
            if unexpected := modules_to_block & sys.modules.keys():
                after = ", ".join(unexpected)
                raise AssertionError(f'unexpectedly imported after additional code: {{after}}')
            """
        )

    from .script_helper import assert_python_ok
    assert_python_ok("-S", "-c", script)
