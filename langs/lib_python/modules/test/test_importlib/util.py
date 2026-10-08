# Derived from CPython v3.14.8 Lib/test/test_importlib/util.py; PSF License.
# Only `uncache` is kept: pkgutil's own test takes it from here, and the
# rest of that module builds on extension modules this runtime does not
# carry.
import contextlib
import sys


@contextlib.contextmanager
def uncache(*names):
    """Uncache a module from sys.modules.

    A basic sanity check is performed to prevent uncaching modules that either
    cannot/shouldn't be uncached.

    """
    for name in names:
        if name in ('sys', 'marshal'):
            raise ValueError("cannot uncache {}".format(name))
        try:
            del sys.modules[name]
        except KeyError:
            pass
    try:
        yield
    finally:
        for name in names:
            try:
                del sys.modules[name]
            except KeyError:
                pass
