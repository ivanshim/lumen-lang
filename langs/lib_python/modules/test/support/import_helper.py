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
