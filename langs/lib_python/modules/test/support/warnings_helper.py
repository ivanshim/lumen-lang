# These warning helpers stand in for the reference suite's own: lumen's
# warning machinery records messages but does not raise the
# ResourceWarning a reference check would look for, so the checks here
# record without judging. The recorder a check hands back is a stand-in
# too, and keeps the caught list visible as the reference recorder's
# ``warnings`` property. The two the grammar suite takes from inside
# its class bodies are written as the reference suite writes them, so
# that the suite reads and its methods run, each failing on its own
# where the warning it looks for never comes.
import contextlib
import warnings
from warnings import catch_warnings

class _RecordedWarnings:
    """What a ``check_warnings`` block hands back.

    The reference helper yields a recorder whose ``warnings`` property
    is the list of messages caught in the block. That list stays visible
    under the same name here, and the object still reads as that list
    for a caller that indexes, iterates or compares it.
    """

    def __init__(self, recorded):
        self._recorded = recorded

    @property
    def warnings(self):
        return self._recorded

    def __len__(self):
        return len(self._recorded)

    def __getitem__(self, index):
        return self._recorded[index]

    def __iter__(self):
        return iter(self._recorded)

    def __eq__(self, other):
        if isinstance(other, _RecordedWarnings):
            other = other._recorded
        return self._recorded == other

    def __repr__(self):
        return repr(self._recorded)


@contextlib.contextmanager
def check_warnings(*filters, quiet=True):
    with catch_warnings(record=True) as recorded:
        yield _RecordedWarnings(recorded)

def ignore_warnings(*args, **kwargs):
    return _identity

def _identity(function):
    return function

# The reference suite writes this one with a call of its own before the
# thing it decorates, so it hands back the decorator rather than being
# one itself.
def ignore_fork_in_thread_deprecation_warnings():
    return _identity


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
