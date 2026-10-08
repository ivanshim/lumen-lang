# These warning helpers stand in for the reference suite's own: no
# warnings are emitted here, so a check that counts them has nothing
# to count. The two the grammar suite takes from inside its class
# bodies are written as the reference suite writes them, so that the
# suite reads and its methods run, each failing on its own where the
# warning it looks for never comes.
import contextlib
import importlib
import re
import sys
import warnings
from warnings import catch_warnings

class WarningsRecorder(object):
    """Convenience wrapper for the warnings list returned on
       entry to the warnings.catch_warnings() context manager.
    """
    def __init__(self, warnings_list):
        self._warnings = warnings_list
        self._last = 0

    def __getattr__(self, attr):
        if len(self._warnings) > self._last:
            return getattr(self._warnings[-1], attr)
        elif attr in warnings.WarningMessage._WARNING_DETAILS:
            return None
        raise AttributeError("%r has no attribute %r" % (self, attr))

    @property
    def warnings(self):
        return self._warnings[self._last:]

    def reset(self):
        self._last = len(self._warnings)


def check_warnings(*filters, **kwargs):
    """Context manager to silence warnings.

    Accept 2-tuples as positional arguments:
        ("message regexp", WarningCategory)

    Optional argument:
     - if 'quiet' is True, it does not fail if a filter catches nothing
        (default True without argument,
         default False if some filters are defined)

    Without argument, it defaults to:
        check_warnings(("", Warning), quiet=True)
    """
    quiet = kwargs.get('quiet')
    if not filters:
        filters = (("", Warning),)
        # Preserve backward compatibility
        if quiet is None:
            quiet = True
    return _filterwarnings(filters, quiet)


@contextlib.contextmanager
def _filterwarnings(filters, quiet=False):
    """Catch the warnings, then check if all the expected
    warnings have been raised and re-raise unexpected warnings.
    If 'quiet' is True, only re-raise the unexpected warnings.
    """
    # Clear the warning registry of the calling module
    # in order to re-raise the warnings.
    frame = sys._getframe(2)
    registry = frame.f_globals.get('__warningregistry__')
    if registry:
        registry.clear()
    # Because test_warnings swap the module, we need to look up in
    # the sys.modules dictionary.
    wmod = sys.modules['warnings']
    with wmod.catch_warnings(record=True) as w:
        # Set filter "always" to record all warnings.
        wmod.simplefilter("always")
        yield WarningsRecorder(w)
    # Filter the recorded warnings
    reraise = list(w)
    missing = []
    for msg, cat in filters:
        seen = False
        for w in reraise[:]:
            warning = w.message
            # Filter out the matching messages
            if (re.match(msg, str(warning), re.I) and
                issubclass(warning.__class__, cat)):
                seen = True
                reraise.remove(w)
        if not seen and not quiet:
            # This filter caught nothing
            missing.append((msg, cat.__name__))
    if reraise:
        raise AssertionError("unhandled warning %s" % reraise[0])
    if missing:
        raise AssertionError("filter (%r, %s) did not catch any warning" %
                             missing[0])

def ignore_warnings(*args, **kwargs):
    return _identity

def _identity(function):
    return function

# The reference suite writes this one with a call of its own before the
# thing it decorates, so it hands back the decorator rather than being
# one itself.
def ignore_fork_in_thread_deprecation_warnings():
    return _identity


def import_deprecated(name):
    """Import *name* while suppressing DeprecationWarning."""
    with warnings.catch_warnings():
        warnings.simplefilter('ignore', category=DeprecationWarning)
        return importlib.import_module(name)


def check_syntax_warning(testcase, statement, errtext='',
                         *, lineno=1, offset=None):
    # Test also that a warning is emitted only once.
    from test.support import check_syntax_error
    with warnings.catch_warnings(record=True) as warns:
        warnings.simplefilter('always', SyntaxWarning)
        compile(statement, '<testcase>', 'exec')
    testcase.assertEqual(len(warns), 1, warns)
    warn, = warns
    testcase.assertTrue(issubclass(warn.category, SyntaxWarning),
                        warn.category)
    if errtext:
        testcase.assertRegex(str(warn.message), errtext)
    testcase.assertEqual(warn.filename, '<testcase>')
    testcase.assertIsNotNone(warn.lineno)
    if lineno is not None:
        testcase.assertEqual(warn.lineno, lineno)

    # SyntaxWarning should be converted to SyntaxError when raised,
    # since the latter contains more information and provides better
    # error report.
    with warnings.catch_warnings(record=True) as warns:
        warnings.simplefilter('error', SyntaxWarning)
        check_syntax_error(testcase, statement, errtext,
                           lineno=lineno, offset=offset)
    # No warnings are leaked when a SyntaxError is raised.
    testcase.assertEqual(warns, [])

@contextlib.contextmanager
def check_no_warnings(testcase, message='', category=Warning, force_gc=False):
    """Context manager to check that no warnings are emitted.

    This context manager enables a given warning within its scope
    and checks that no warnings are emitted even with that warning
    enabled.

    If force_gc is True, a garbage collection is attempted before checking
    for warnings. This may help to catch warnings emitted when objects
    are deleted, such as ResourceWarning.

    Other keyword arguments are passed to warnings.filterwarnings().
    """
    from test.support import gc_collect
    with warnings.catch_warnings(record=True) as warns:
        warnings.filterwarnings('always',
                                message=message,
                                category=category)
        yield
        if force_gc:
            gc_collect()
    testcase.assertEqual(warns, [])

@contextlib.contextmanager
def save_restore_warnings_filters():
    old_filters = warnings.filters[:]
    try:
        yield
    finally:
        warnings.filters[:] = old_filters
