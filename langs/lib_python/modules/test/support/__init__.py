# Environment decorators which ask about the environment we have are
# stubs: they retain the decorated function. Those which ask whether
# this is the reference implementation answer no and skip, because a
# test of another implementation's internals says nothing about this one
# whichever way it comes out.
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

# A guard names the implementations a test is for, all together wanted
# or all together unwanted; an implementation it does not name gets the
# opposite answer. Naming none of them means the reference one.
def _parse_guards(guards):
    if not guards:
        return ({'cpython': True}, False)
    wanted = list(guards.values())[0]
    return (guards, not wanted)

def check_impl_detail(**guards):
    named, otherwise = _parse_guards(guards)
    return named.get(sys.implementation.name, otherwise)

def impl_detail(message=None, **guards):
    if check_impl_detail(**guards):
        return _identity
    if message is None:
        named, otherwise = _parse_guards(guards)
        names = ' or '.join(sorted(named))
        if otherwise:
            message = 'implementation detail not available on ' + names
        else:
            message = 'implementation detail specific to ' + names
    return unittest.skip(message)

# Tests of the reference implementation's own internals -- which tuple
# it hands back a second time, how many names hold a value, what its C
# code does at the edges. This is not that implementation, so running
# them proves nothing: they pass or fail on how this kernel happens to
# allocate, and either answer is an accident.
def cpython_only(test):
    return impl_detail(cpython=True)(test)

# Reference counting is that implementation's own way of knowing when a
# value is finished with. There is none here to count, so these belong
# with the rest of its internals.
refcount_test = cpython_only

# The memory-exhaustion tests turn off that implementation's allocator
# through its own C test module, so they are its internals too.
nomemtest = cpython_only

# This kernel's floats are the host's binary64, so the tests which ask
# for IEEE 754 doubles are asking for what they get and simply run.
requires_IEEE_754 = _identity

# Only Cygwin's newlib C library fails these, and the host is not it.
skip_on_newlib = _identity

def requires_subprocess():
    return unittest.skip('subprocesses are not supported')

# Nothing here colours its output, so a class asking for plain output
# already has it and runs unchanged.
force_not_colorized_test_class = _identity

def is_resource_enabled(resource):
    return False

def requires_resource(resource):
    return unittest.skipUnless(is_resource_enabled(resource), 'resource ' + resource + ' is not enabled')

# A note for a runner that would put tests in threads at once. Nothing
# here does, so the note is kept and the test runs.
def thread_unsafe(reason=''):
    return _identity

def bigmemtest(size, memuse, dry_run=True):
    return _SmallMemory(size, memuse, dry_run).decorate

def requires_mac_ver(*version):
    return _identity

def run_with_locale(*locales):
    return _identity

def run_with_limited_c_stack(*args, **kwargs):
    return _identity

def skip_wasi_stack_overflow():
    return _identity

def skip_emscripten_stack_overflow():
    return _identity

def skip_if_huge_c_stack():
    return _identity

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

# The digit limit is set for the block and put back after it, whatever
# the block did.
class adjust_int_max_str_digits:
    def __init__(self, max_digits):
        self.max_digits = max_digits

    def __enter__(self):
        self.old = sys.get_int_max_str_digits()
        sys.set_int_max_str_digits(self.max_digits)
        return self

    def __exit__(self, kind, value, traceback):
        sys.set_int_max_str_digits(self.old)
        return False

# The printer follows whatever sys holds as its stream, so capturing is
# putting a StringIO in the stream's place and taking it out again.
class captured_stdout:
    def __enter__(self):
        self.saved = sys.stdout
        self.stream = StringIO()
        sys.stdout = self.stream
        return self.stream

    def __exit__(self, kind, value, traceback):
        sys.stdout = self.saved
        return False

class captured_stderr:
    def __enter__(self):
        self.saved = sys.stderr
        self.stream = StringIO()
        sys.stderr = self.stream
        return self.stream

    def __exit__(self, kind, value, traceback):
        sys.stderr = self.saved
        return False

def _unavailable(*args, **kwargs):
    raise 'NotImplementedError: this test support helper is not supported'

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
force_not_colorized = _identity
skip_if_double_rounding = _identity
_1G = 1073741824
_2G = 2147483648
_4G = 4294967296
TestFailed = unittest.AssertionError

class _NeverEqual:
    def __eq__(self, other):
        return False

    def __ne__(self, other):
        return True

    # Equality of its own leaves a class without a hash; this one keeps
    # a hash of its own so that its value may be a key or a member.
    def __hash__(self):
        return 1

NEVER_EQ = _NeverEqual()

# Tracing control is a stub; the kernel does not install trace callbacks.
no_tracing = _identity
SuppressCrashReport = _unavailable

# The dry run uses the small trial size of the reference suite. A test
# refusing a dry run needs the resource explicitly enabled.
class _SmallMemory:
    def __init__(self, size, memuse, dry_run):
        self.size = size
        self.memuse = memuse
        self.dry_run = dry_run

    def decorate(self, function):
        self.function = function
        return self.call

    def call(self, testcase):
        if not self.dry_run:
            raise unittest.SkipTest('big-memory resource is not enabled')
        return self.function(testcase, 5147)

_1M = 1048576
HAVE_DOCSTRINGS = False

class _Sentinel:
    pass

sentinel = _Sentinel()

from test.support.import_helper import import_module, import_fresh_module

def skip_if_sanitizer(reason=None, **sanitizers):
    return _identity
