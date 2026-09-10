# Environment decorators are stubs: they retain the decorated function.
# Helpers whose work is unavailable complain when called, never pass a test.
import gc
import sys
import unittest
from io import StringIO

MISSING_C_DOCSTRINGS = True
verbose = False
is_wasi = False
is_emscripten = False
Py_DEBUG = False
Py_GIL_DISABLED = False
MS_WINDOWS = False
MAX_Py_ssize_t = sys.maxsize
NHASHBITS = 64
_TPFLAGS_BASETYPE = 1024
_TPFLAGS_HAVE_GC = 16384
_TPFLAGS_IMMUTABLETYPE = 256

def _identity(function):
    return function

cpython_only = _identity
requires_IEEE_754 = _identity
skip_on_newlib = _identity
nomemtest = _identity
refcount_test = _identity
requires_subprocess = _identity
force_not_colorized_test_class = _identity

def requires_resource(resource):
    # Stub: resource selection belongs to the enclosing suite runner.
    return _identity

def thread_unsafe(reason=''):
    return _identity

def bigmemtest(size, memuse, dry_run=True):
    # Supplying a made-up size would silently change the test's question.
    raise 'NotImplementedError: big-memory test argument injection is not supported'

def requires_mac_ver(*version):
    return _identity

def run_with_locale(*locales):
    raise 'NotImplementedError: locale changes are not supported'

def run_with_limited_c_stack(*args, **kwargs):
    return _identity

def skip_wasi_stack_overflow():
    return _identity

def skip_emscripten_stack_overflow():
    return _identity

def skip_if_huge_c_stack(function):
    return function

def linked_to_musl():
    # Stub: no host C library is inspected.
    return None

def gc_collect():
    return gc.collect()

def run_unittest(*classes):
    result = unittest.TestResult()
    loader = unittest.TestLoader()
    for cls in classes:
        loader.loadTestsFromTestCase(cls).run(result)
    if not result.wasSuccessful():
        raise unittest.AssertionError('unittest suite failed')
    return result

def check_syntax_error(testcase, statement, errtext='', lineno=None, offset=None):
    raise 'NotImplementedError: syntax checks need a compile builtin'

class swap_attr:
    def __init__(self, obj, attr, new_val):
        self.obj = obj
        self.attr = attr
        self.new_val = new_val

    def __enter__(self):
        self.old = getattr(self.obj, self.attr)
        setattr(self.obj, self.attr, self.new_val)
        return self.old

    def __exit__(self, kind, value, traceback):
        setattr(self.obj, self.attr, self.old)
        return False

class captured_stdout:
    def __enter__(self):
        import sys, io
        self.saved = sys.stdout
        self.stream = io.StringIO()
        sys.stdout = self.stream
        return self.stream

    def __exit__(self, kind, value, traceback):
        import sys
        sys.stdout = self.saved
        return False

class captured_stderr:
    def __enter__(self):
        import sys, io
        self.saved = sys.stderr
        self.stream = io.StringIO()
        sys.stderr = self.stream
        return self.stream

    def __exit__(self, kind, value, traceback):
        import sys
        sys.stderr = self.saved
        return False

def check_impl_detail(**guards):
    # Stub: this run claims no reference implementation internals.
    return False

def _unavailable(*args, **kwargs):
    raise 'NotImplementedError: this test support helper is not supported'

adjust_int_max_str_digits = _unavailable
calcobjsize = _unavailable
catch_unraisable_exception = _unavailable
check_free_after_iterating = _unavailable
collision_stats = _unavailable
findfile = _unavailable
run_in_subinterp = _unavailable
set_memlimit = _unavailable
swap_item = _unavailable
wait_process = _unavailable
Stopwatch = _unavailable

def exceeds_recursion_limit():
    return sys.getrecursionlimit() + 100

class BrokenIter:
    def __iter__(self):
        raise 'RuntimeError: broken iterator'

class _AlwaysEqual:
    def __eq__(self, other):
        return True

# Comparison dispatch is not yet honoured for user objects. The value
# remains visible, so uses which require it will meet that limitation.
ALWAYS_EQ = _AlwaysEqual()

# These entry points can be imported, but their absent machinery must
# be named before a test can mistake it for a successful check.
async_yield = _unavailable
run_yielding_async_fn = _unavailable
force_not_colorized = _unavailable
skip_if_double_rounding = _identity
_1G = 1073741824
_2G = 2147483648
_4G = 4294967296
TestFailed = unittest.AssertionError

class _NeverEqual:
    def __eq__(self, other):
        return False

NEVER_EQ = _NeverEqual()

# Tracing control is a stub; the kernel does not install trace callbacks.
no_tracing = _identity
SuppressCrashReport = _unavailable
