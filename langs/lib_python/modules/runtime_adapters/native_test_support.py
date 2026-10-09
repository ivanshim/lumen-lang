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

# Nothing stands behind os.fork in this library yet; a test that needs
# a forked process says so by stepping aside.

# The memory-exhaustion tests turn off that implementation's allocator
# through its own C test module, so they are its internals too.
nomemtest = cpython_only

# This kernel's floats are the host's binary64, so the tests which ask
# for IEEE 754 doubles are asking for what they get and simply run.
requires_IEEE_754 = _identity

# Only Cygwin's newlib C library fails these, and the host is not it.
skip_on_newlib = _identity


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
    gc.collect()
    gc.collect()
    gc.collect()

def run_unittest(*classes):
    result = unittest.TestResult()
    loader = unittest.TestLoader()
    for cls in classes:
        loader.loadTestsFromTestCase(cls).run(result)
    if not result.wasSuccessful():
        raise AssertionError('unittest suite failed')
    return result

def check_syntax_error(testcase, statement, errtext='', lineno=None, offset=None):
    with testcase.assertRaisesRegex(SyntaxError, errtext) as caught:
        compile(statement, '<test string>', 'exec')
    if lineno is not None:
        testcase.assertEqual(caught.exception.lineno, lineno)
    if offset is not None:
        testcase.assertEqual(caught.exception.offset, offset)

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

# A thing greater than everything but itself, and one smaller than
# everything but itself; the reference builds its pair out of
# total_ordering, and these are the answers that falls out of.
class _Largest:
    def __eq__(self, other): return isinstance(other, _Largest)
    def __ne__(self, other): return not isinstance(other, _Largest)
    def __lt__(self, other): return False
    def __le__(self, other): return isinstance(other, _Largest)
    def __gt__(self, other): return not isinstance(other, _Largest)
    def __ge__(self, other): return True

LARGEST = _Largest()

class _Smallest:
    def __eq__(self, other): return isinstance(other, _Smallest)
    def __ne__(self, other): return not isinstance(other, _Smallest)
    def __gt__(self, other): return False
    def __ge__(self, other): return isinstance(other, _Smallest)
    def __lt__(self, other): return not isinstance(other, _Smallest)
    def __le__(self, other): return True

SMALLEST = _Smallest()

# These entry points can be imported, but their absent machinery must
# be named before a test can mistake it for a successful check.
force_not_colorized = _identity
skip_if_double_rounding = _identity
_1G = 1073741824
_2G = 2147483648
_4G = 4294967296
TestFailed = AssertionError

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

# The dry run uses the small trial size of the reference suite. A test
# refusing a dry run needs the resource explicitly enabled.
class _SmallMemory:
    def __init__(self, size, memuse, dry_run):
        self.size = size
        self.memuse = memuse
        self.dry_run = dry_run

    def decorate(self, function):
        memory = self
        def wrapper(testcase):
            size = memory.size if real_max_memuse else 5147
            if (real_max_memuse or not memory.dry_run) and real_max_memuse < size * memory.memuse:
                raise unittest.SkipTest('not enough memory: %.1fG minimum needed' %
                                        (memory.size * memory.memuse / (1024 ** 3)))
            return function(testcase, size)
        wrapper.memuse = self.memuse
        wrapper.size = self.size
        return wrapper

_1M = 1048576
HAVE_DOCSTRINGS = False

class _Sentinel:
    pass

sentinel = _Sentinel()

from test.support.import_helper import import_module, import_fresh_module

def skip_if_sanitizer(reason=None, **sanitizers):
    return _identity

# The embedded test package corresponds to the reference files beside
# the executable's source tree, even when the caller changes directory.
import os
import time
import re
TEST_HOME_DIR = os.path.join(os.path.dirname(os.path.dirname(os.path.dirname(os.path.dirname(os.path.dirname(os.path.dirname(__file__)))))), 'tests', 'python-3.14.8')
REPO_ROOT = os.path.dirname(os.path.dirname(TEST_HOME_DIR))
_header = 'nP'
_align = '0n'
LOOPBACK_TIMEOUT = 10.0
INTERNET_TIMEOUT = 60.0
SHORT_TIMEOUT = 30.0
LONG_TIMEOUT = 300.0
max_memuse = 0
real_max_memuse = 0

def _check_tracemalloc():
    try:
        import tracemalloc
    except ImportError:
        return
    if tracemalloc.is_tracing():
        raise unittest.SkipTest('run_in_subinterp() cannot be used if tracemalloc module is tracing memory allocations')

def findfile(filename, subdir=None):
    if filename.startswith('/'):
        return filename
    if subdir is not None:
        filename = os.path.join(subdir, filename)
    # Beside the reference files themselves, test data a reference test
    # reaches for lives with the embedded test package, laid out as the
    # reference lays it out beside its own tests.
    embedded_test_home = os.path.dirname(os.path.dirname(__file__))
    path = [TEST_HOME_DIR, embedded_test_home] + sys.path
    for dn in path:
        fn = os.path.join(dn, filename)
        if os.path.exists(fn): return fn
    return filename

class disable_gc:
    def __enter__(self):
        self.enabled = gc.isenabled()
        gc.disable()

    def __exit__(self, *exc):
        if self.enabled:
            gc.enable()
        return False

def calcobjsize(fmt):
    import struct
    return struct.calcsize(_header + fmt + _align)

def _parse_memlimit(limit: str) -> int:
    sizes = {
        'k': 1024,
        'm': _1M,
        'g': _1G,
        't': 1024*_1G,
    }
    m = re.match(r'(\d+(?:\.\d+)?) (K|M|G|T)b?$', limit,
                 re.IGNORECASE | re.VERBOSE)
    if m is None:
        raise ValueError(f'Invalid memory limit: {limit!r}')
    return int(float(m.group(1)) * sizes[m.group(2).lower()])

def set_memlimit(limit: str) -> None:
    global max_memuse
    global real_max_memuse
    memlimit = _parse_memlimit(limit)
    if memlimit < _2G - 1:
        raise ValueError(f'Memory limit {limit!r} too low to be useful')

    real_max_memuse = memlimit
    memlimit = min(memlimit, MAX_Py_ssize_t)
    max_memuse = memlimit

class swap_item:
    def __init__(self, obj, item, new_val):
        self.obj = obj
        self.item = item
        self.new_val = new_val

    def __enter__(self):
        self.present = self.item in self.obj
        self.old = self.obj[self.item] if self.present else None
        self.obj[self.item] = self.new_val
        return self.old

    def __exit__(self, *exc):
        if self.present:
            self.obj[self.item] = self.old
        elif self.item in self.obj:
            del self.obj[self.item]
        return False

class SuppressCrashReport:
    old_value = None
    old_modes = None

    def __enter__(self):
        if sys.platform.startswith('win'):
            # see http://msdn.microsoft.com/en-us/library/windows/desktop/ms680621.aspx
            try:
                import msvcrt
            except ImportError:
                return

            self.old_value = msvcrt.GetErrorMode()

            msvcrt.SetErrorMode(self.old_value | msvcrt.SEM_NOGPFAULTERRORBOX)

            # bpo-23314: Suppress assert dialogs in debug builds.
            # CrtSetReportMode() is only available in debug build.
            if hasattr(msvcrt, 'CrtSetReportMode'):
                self.old_modes = {}
                for report_type in [msvcrt.CRT_WARN,
                                    msvcrt.CRT_ERROR,
                                    msvcrt.CRT_ASSERT]:
                    old_mode = msvcrt.CrtSetReportMode(report_type,
                            msvcrt.CRTDBG_MODE_FILE)
                    old_file = msvcrt.CrtSetReportFile(report_type,
                            msvcrt.CRTDBG_FILE_STDERR)
                    self.old_modes[report_type] = old_mode, old_file

        else:
            try:
                import resource
                self.resource = resource
            except ImportError:
                raise unittest.SkipTest("No module named 'resource'")
            if self.resource is not None:
                try:
                    self.old_value = self.resource.getrlimit(self.resource.RLIMIT_CORE)
                    self.resource.setrlimit(self.resource.RLIMIT_CORE,
                                            (0, self.old_value[1]))
                except (ValueError, OSError):
                    pass

            if sys.platform == 'darwin':
                import subprocess
                # Check if the 'Crash Reporter' on OSX was configured
                # in 'Developer' mode and warn that it will get triggered
                # when it is.
                #
                # This assumes that this context manager is used in tests
                # that might trigger the next manager.
                cmd = ['/usr/bin/defaults', 'read',
                       'com.apple.CrashReporter', 'DialogType']
                proc = subprocess.Popen(cmd,
                                        stdout=subprocess.PIPE,
                                        stderr=subprocess.PIPE)
                with proc:
                    stdout = proc.communicate()[0]
                if stdout.strip() == b'developer':
                    print("this test triggers the Crash Reporter, "
                          "that is intentional", end='', flush=True)

        return self

    def __exit__(self, *ignore_exc):
        if self.old_value is None:
            return

        if sys.platform.startswith('win'):
            import msvcrt
            msvcrt.SetErrorMode(self.old_value)

            if self.old_modes:
                for report_type, (old_mode, old_file) in self.old_modes.items():
                    msvcrt.CrtSetReportMode(report_type, old_mode)
                    msvcrt.CrtSetReportFile(report_type, old_file)
        else:
            if self.resource is not None:
                try:
                    self.resource.setrlimit(self.resource.RLIMIT_CORE, self.old_value)
                except (ValueError, OSError):
                    pass

def run_in_subinterp(code):
    _check_tracemalloc()
    try:
        import _testcapi
    except ImportError:
        raise unittest.SkipTest("requires _testcapi")
    return _testcapi.run_in_subinterp(code)

def check_free_after_iterating(test, iter, cls, args=()):
    done = False
    def wrapper():
        class A(cls):
            def __del__(self):
                nonlocal done
                done = True
                try:
                    next(it)
                except StopIteration:
                    pass

        it = iter(A(*args))
        # Issue 26494: Shouldn't crash
        test.assertRaises(StopIteration, next, it)

    wrapper()
    # The sequence should be deallocated just after the end of iterating
    gc_collect()
    test.assertTrue(done)

def collision_stats(nbins, nballs):
    n, k = nbins, nballs
    if n > 0 and k == 1:
        return 0.0, 0.0
    # prob a bin empty after k trials = (1 - 1/n)**k
    # mean # empty is then n * (1 - 1/n)**k
    # so mean # occupied is n - n * (1 - 1/n)**k
    # so collisions = k - (n - n*(1 - 1/n)**k)
    #
    # For the variance:
    # n*(n-1)*(1-2/n)**k + meanempty - meanempty**2 =
    # n*(n-1)*(1-2/n)**k + meanempty * (1 - meanempty)
    #
    # Massive cancellation occurs, and, e.g., for a 64-bit hash code
    # 1-1/2**64 rounds uselessly to 1.0. Keep extra precision through
    # the cancellations before converting the result to binary64.
    #
    # Note:  the exact values are straightforward to compute with
    # rationals, but in context that's unbearably slow, requiring
    # multi-million bit arithmetic.
    import math
    # Fixed-point evaluation of the same empty-bin probabilities. The
    # scale leaves enough guard digits for cancellation of n squared
    # and for the accumulated error in k multiplications.
    scale = 10 ** (max(n.bit_length() * 2, 30) + k.bit_length() + 10)
    def probability(empty):
        base = (n - empty) * scale // n
        exponent = k
        result = scale
        while exponent:
            if exponent & 1:
                result = result * base // scale
            exponent >>= 1
            if exponent:
                base = base * base // scale
        return result
    meanempty = n * probability(1)
    collisions = (k - n) * scale + meanempty
    variance = n * (n - 1) * probability(2) + meanempty - meanempty * meanempty // scale
    return collisions / scale, math.sqrt(variance / scale)

class catch_unraisable_exception:

    def __init__(self):
        self.unraisable = None
        self._old_hook = None

    def _hook(self, unraisable):
        # Storing unraisable.object can resurrect an object which is being
        # finalized. Storing unraisable.exc_value creates a reference cycle.
        self.unraisable = unraisable

    def __enter__(self):
        self._old_hook = sys.unraisablehook
        sys.unraisablehook = self._hook
        return self

    def __exit__(self, *exc_info):
        sys.unraisablehook = self._old_hook
        del self.unraisable

def wait_process(pid, *, exitcode, timeout=None):
    if not hasattr(os, 'waitpid'):
        raise unittest.SkipTest('requires subprocess support')
    if os.name != "nt":
        import signal

        if timeout is None:
            timeout = LONG_TIMEOUT

        start_time = time.monotonic()
        for _ in sleeping_retry(timeout, error=False):
            pid2, status = os.waitpid(pid, os.WNOHANG)
            if pid2 != 0:
                break
            # Retry: the process is still running
        else:
            try:
                os.kill(pid, signal.SIGKILL)
                os.waitpid(pid, 0)
            except OSError:
                # Ignore errors like ChildProcessError or PermissionError
                pass

            dt = time.monotonic() - start_time
            raise AssertionError(f"process {pid} is still running "
                                 f"after {dt:.1f} seconds")
    else:
        # Windows implementation: don't support timeout :-(
        pid2, status = os.waitpid(pid, 0)

    exitcode2 = os.waitstatus_to_exitcode(status)
    if exitcode2 != exitcode:
        raise AssertionError(f"process {pid} exited with code {exitcode2}, "
                             f"but exit code {exitcode} is expected")

    # sanity check: it should not fail in practice
    if pid2 != pid:
        raise AssertionError(f"pid {pid2} != pid {pid}")

def busy_retry(timeout, err_msg=None, /, *, error=True):
    if timeout <= 0:
        raise ValueError("timeout must be greater than zero")

    start_time = time.monotonic()
    deadline = start_time + timeout

    while True:
        yield

        if time.monotonic() >= deadline:
            break

    if error:
        dt = time.monotonic() - start_time
        msg = f"timeout ({dt:.1f} seconds)"
        if err_msg:
            msg = f"{msg}: {err_msg}"
        raise AssertionError(msg)

def sleeping_retry(timeout, err_msg=None, /,
                     *, init_delay=0.010, max_delay=1.0, error=True):

    delay = init_delay
    for _ in busy_retry(timeout, err_msg, error=error):
        yield

        time.sleep(delay)
        delay = min(delay * 2, max_delay)

class Stopwatch:
    def __enter__(self):
        get_time = time.perf_counter
        clock_info = time.get_clock_info('perf_counter')
        self.context = disable_gc()
        self.context.__enter__()
        self.get_time = get_time
        self.clock_info = clock_info
        self.start_time = get_time()
        return self

    def __exit__(self, *exc):
        try:
            end_time = self.get_time()
        finally:
            result = self.context.__exit__(*exc)
        self.seconds = end_time - self.start_time
        return result

class _AsyncYield:
    def __init__(self, value):
        self.value = value
        self.iterator = self._yield()

    def _yield(self):
        return (yield self.value)

    def __await__(self):
        return self.iterator

    def __iter__(self):
        return self.iterator

    def __next__(self):
        return next(self.iterator)

    def send(self, value):
        return self.iterator.send(value)

    def throw(self, *args):
        return self.iterator.throw(*args)

    def close(self):
        return self.iterator.close()


def async_yield(v):
    return _AsyncYield(v)

def run_yielding_async_fn(async_fn, /, *args, **kwargs):
    coro = async_fn(*args, **kwargs)
    try:
        while True:
            try:
                coro.send(None)
            except StopIteration as e:
                return e.value
    finally:
        coro.close()

# These conditions concern platforms and docstrings, not C internals.
skip_on_s390x = _identity
def _check_docstrings():
    """A function whose docstring checks whether documentation is retained."""
    pass

HAVE_PY_DOCSTRINGS = _check_docstrings.__doc__ is not None
requires_docstrings = unittest.skipUnless(HAVE_DOCSTRINGS, "docstrings are required")

def skip_if_unlimited_stack_size(test):
    return test

# Runtime adapter derived from CPython v3.14.8 / 8e6e75d9102e, Lib/test/support/__init__.py; PSF License.
import annotationlib
class EqualToForwardRef:
    """Helper to ease use of annotationlib.ForwardRef in tests.

    This checks only attributes that can be set using the constructor.

    """

    def __init__(
        self,
        arg,
        *,
        module=None,
        owner=None,
        is_class=False,
    ):
        self.__forward_arg__ = arg
        self.__forward_is_class__ = is_class
        self.__forward_module__ = module
        self.__owner__ = owner

    def __eq__(self, other):
        if not isinstance(other, (EqualToForwardRef, annotationlib.ForwardRef)):
            return NotImplemented
        return (
            self.__forward_arg__ == other.__forward_arg__
            and self.__forward_module__ == other.__forward_module__
            and self.__forward_is_class__ == other.__forward_is_class__
            and self.__owner__ == other.__owner__
        )

    def __repr__(self):
        extra = []
        if self.__forward_module__ is not None:
            extra.append(f", module={self.__forward_module__!r}")
        if self.__forward_is_class__:
            extra.append(", is_class=True")
        if self.__owner__ is not None:
            extra.append(f", owner={self.__owner__!r}")
        return f"EqualToForwardRef({self.__forward_arg__!r}{''.join(extra)})"

# Runtime adapter derived from CPython Lib/test/support/__init__.py at v3.14.8 / 8e6e75d9102e; PSF License.
def check__all__(test_case, module, name_of_module=None, extra=(),
                 not_exported=()):
    """Assert that the __all__ variable of 'module' contains all public names.

    The module's public names (its API) are detected automatically based on
    whether they match the public name convention and were defined in
    'module'.

    The 'name_of_module' argument can specify (as a string or tuple thereof)
    what module(s) an API could be defined in order to be detected as a
    public API. One case for this is when 'module' imports part of its public
    API from other modules, possibly a C backend (like 'csv' and its '_csv').

    The 'extra' argument can be a set of names that wouldn't otherwise be
    automatically detected as "public", like objects without a proper
    '__module__' attribute. If provided, it will be added to the
    automatically detected ones.

    The 'not_exported' argument can be a set of names that must not be treated
    as part of the public API even though their names indicate otherwise.

    Usage:
        import bar
        import foo
        import unittest
        from test import support

        class MiscTestCase(unittest.TestCase):
            def test__all__(self):
                support.check__all__(self, foo)

        class OtherTestCase(unittest.TestCase):
            def test__all__(self):
                extra = {'BAR_CONST', 'FOO_CONST'}
                not_exported = {'baz'}  # Undocumented name.
                # bar imports part of its API from _bar.
                support.check__all__(self, bar, ('bar', '_bar'),
                                     extra=extra, not_exported=not_exported)

    """

    if name_of_module is None:
        name_of_module = (module.__name__, )
    elif isinstance(name_of_module, str):
        name_of_module = (name_of_module, )

    import types
    expected = set(extra)

    for name in dir(module):
        if name.startswith('_') or name in not_exported:
            continue
        obj = getattr(module, name)
        if (getattr(obj, '__module__', None) in name_of_module or
                (not hasattr(obj, '__module__') and
                 not isinstance(obj, types.ModuleType))):
            expected.add(name)
    test_case.assertCountEqual(module.__all__, expected)



# From CPython Lib/test/support at v3.14.8 / 8e6e75d9102e, PSF License.
import functools
PGO = False
PGO_EXTENDED = False

def check_sizeof(test, o, size):
    try:
        import _testinternalcapi
    except ImportError:
        raise unittest.SkipTest("_testinternalcapi required")
    result = sys.getsizeof(o)
    # add GC header size
    if ((type(o) == type) and (o.__flags__ & _TPFLAGS_HEAPTYPE) or\
        ((type(o) != type) and (type(o).__flags__ & _TPFLAGS_HAVE_GC))):
        size += _testinternalcapi.SIZEOF_PYGC_HEAD
    msg = 'wrong size for %s: got %d, expected %d' \
            % (type(o), result, size)
    test.assertEqual(result, size, msg)

def bigaddrspacetest(f):
    """Decorator for tests that fill the address space."""
    @functools.wraps(f)
    def wrapper(self):
        if max_memuse < MAX_Py_ssize_t:
            if MAX_Py_ssize_t >= 2**63 - 1 and max_memuse >= 2**31:
                raise unittest.SkipTest(
                    "not enough memory: try a 32-bit build instead")
            else:
                raise unittest.SkipTest(
                    "not enough memory: %.1fG minimum needed"
                    % (MAX_Py_ssize_t / (1024 ** 3)))
        else:
            return f(self)
    return wrapper

def skip_if_pgo_task(test):
    """Skip decorator for tests not run in (non-extended) PGO task"""
    ok = not PGO or PGO_EXTENDED
    msg = "Not run for (non-extended) PGO task"
    return test if ok else unittest.skip(msg)(test)

def check_immutable_type(testcase, type):
    regex = r'cannot set .* attribute of immutable type'
    with testcase.assertRaisesRegex(TypeError, regex):
        setattr(type, 'custom_attr', 123)

    try:
        from _testlimitedcapi import type_getflags, Py_TPFLAGS_IMMUTABLETYPE
    except ImportError:
        pass
    else:
        flags = type_getflags(type)
        testcase.assertTrue(flags & Py_TPFLAGS_IMMUTABLETYPE)

import contextlib

@contextlib.contextmanager
def captured_output(stream_name):
    """Return a context manager used by captured_stdout/stdin/stderr
    that temporarily replaces the sys stream *stream_name* with a StringIO."""
    import io
    orig_stdout = getattr(sys, stream_name)
    setattr(sys, stream_name, io.StringIO())
    try:
        yield getattr(sys, stream_name)
    finally:
        setattr(sys, stream_name, orig_stdout)

is_apple = sys.platform in ("darwin", "ios", "tvos", "watchos")

# CPython v3.14.8 Lib/test/support/__init__.py; PSF License.
@contextlib.contextmanager
def swap_attr(obj, attr, new_val):
    """Temporary swap out an attribute with a new object.

    Usage:
        with swap_attr(obj, "attr", 5):
            ...

        This will set obj.attr to 5 for the duration of the with: block,
        restoring the old value at the end of the block. If `attr` doesn't
        exist on `obj`, it will be created and then deleted at the end of the
        block.

        The old value (or None if it doesn't exist) will be assigned to the
        target of the "as" clause, if there is one.
    """
    if hasattr(obj, attr):
        real_val = getattr(obj, attr)
        setattr(obj, attr, new_val)
        try:
            yield real_val
        finally:
            setattr(obj, attr, real_val)
    else:
        setattr(obj, attr, new_val)
        try:
            yield
        finally:
            if hasattr(obj, attr):
                delattr(obj, attr)
# Derived from CPython v3.14.8 Lib/test/support; PSF License.
import os
import re
import textwrap
import types
import time

# Embedded modules use the test archive's package root for discovery.
SHORT_TIMEOUT = 30.0
STDLIB_DIR = TEST_HOME_DIR
TEST_SUPPORT_DIR = os.path.join(TEST_HOME_DIR, 'test', 'support')
is_android = False
is_apple = sys.platform in ("darwin", "ios", "tvos", "watchos")
has_socket_support = False
has_fork_support = False
has_subprocess_support = True
can_start_thread = False
_TPFLAGS_STATIC_BUILTIN = 1 << 1
_TPFLAGS_HEAPTYPE = 1 << 9
_vheader = _header + 'n'

def load_package_tests(pkg_dir, loader, standard_tests, pattern):
    """Generic load_tests implementation for simple test packages.

    Most packages can implement load_tests using this function as follows:

       def load_tests(*args):
           return load_package_tests(os.path.dirname(__file__), *args)
    """
    if pattern is None:
        pattern = "test*"
    top_dir = STDLIB_DIR
    # Reference packages live in the interpreter's versioned test directory.
    if not os.path.abspath(pkg_dir).startswith(os.path.abspath(top_dir).rstrip('/') + '/'):
        top_dir = os.path.dirname(pkg_dir)
    try:
        package_tests = loader.discover(start_dir=pkg_dir,
                                        top_level_dir=top_dir,
                                        pattern=pattern)
    except NotImplementedError:
        package_tests = _discover_package_tests(pkg_dir, loader, top_dir, pattern)
    standard_tests.addTests(package_tests)
    return standard_tests


def requires_zlib(reason='requires zlib'):
    try:
        import zlib
    except ImportError:
        zlib = None
    return unittest.skipUnless(zlib, reason)


def requires_fork():
    return unittest.skipUnless(has_fork_support, "requires working os.fork()")


def requires_subprocess():
    """Used for subprocess, os.spawn calls, fd inheritance"""
    return unittest.skipUnless(has_subprocess_support, "requires subprocess support")


def requires_working_socket(*, module=False):
    """Skip tests or modules that require working sockets

    Can be used as a function/class decorator or to skip an entire module.
    """
    msg = "requires socket support"
    if module:
        if not has_socket_support:
            raise unittest.SkipTest(msg)
    else:
        return unittest.skipUnless(has_socket_support, msg)


def run_code(code: str, extra_names: dict[str, object] | None = None) -> dict[str, object]:
    """Run a piece of code after dedenting it, and return its global namespace."""
    ns = {}
    if extra_names:
        ns.update(extra_names)
    exec(textwrap.dedent(code), ns)
    return ns


def captured_stdin():
    """Capture the input to sys.stdin:

       with captured_stdin() as stdin:
           stdin.write('hello\\n')
           stdin.seek(0)
           # call test code that consumes from sys.stdin
           captured = input()
       self.assertEqual(captured, "hello")
    """
    return captured_output("stdin")


def calcvobjsize(fmt):
    import struct
    return struct.calcsize(_vheader + fmt + _align)


def subTests(arg_names, arg_values, /, *, _do_cleanups=False):
    """Run multiple subtests with different parameters.
    """
    single_param = False
    if isinstance(arg_names, str):
        arg_names = arg_names.replace(',',' ').split()
        if len(arg_names) == 1:
            single_param = True
    arg_values = tuple(arg_values)
    def decorator(func):
        if isinstance(func, type):
            raise TypeError('subTests() can only decorate methods, not classes')

        def iter_subtest_kwargs():
            for values in arg_values:
                yield dict(zip(arg_names, (values,) if single_param else values))

        # A synchronous wrapper would discard the coroutine without awaiting
        # it, so an asynchronous test would not run at all.
        import inspect
        if inspect.iscoroutinefunction(func):
            @functools.wraps(func)
            async def wrapper(self, /, *args, **kwargs):
                for subtest_kwargs in iter_subtest_kwargs():
                    with self.subTest(**subtest_kwargs):
                        await func(self, *args, **kwargs, **subtest_kwargs)
                    if _do_cleanups:
                        self.doCleanups()
        else:
            @functools.wraps(func)
            def wrapper(self, /, *args, **kwargs):
                for subtest_kwargs in iter_subtest_kwargs():
                    with self.subTest(**subtest_kwargs):
                        func(self, *args, **kwargs, **subtest_kwargs)
                    if _do_cleanups:
                        self.doCleanups()
        return wrapper
    return decorator


def run_with_locales(catstr, *locales):
    def deco(func):
        @functools.wraps(func)
        def wrapper(self, /, *args, **kwargs):
            dry_run = '' in locales
            try:
                import locale
                category = getattr(locale, catstr)
                orig_locale = locale.setlocale(category)
            except AttributeError:
                # if the test author gives us an invalid category string
                raise
            except Exception:
                # cannot retrieve original locale, so do nothing
                pass
            else:
                try:
                    for loc in locales:
                        with self.subTest(locale=loc):
                            try:
                                locale.setlocale(category, loc)
                            except (getattr(locale, "Error", ValueError), NotImplementedError):
                                self.skipTest(f'no locale {loc!r}')
                            else:
                                dry_run = False
                                func(self, *args, **kwargs)
                finally:
                    locale.setlocale(category, orig_locale)
            if dry_run:
                # no locales available, so just run the test
                # with the current locale
                with self.subTest(locale=None):
                    func(self, *args, **kwargs)
        return wrapper
    return deco


def run_with_tz(tz):
    def decorator(func):
        def inner(*args, **kwds):
            try:
                tzset = time.tzset
            except AttributeError:
                raise unittest.SkipTest("tzset required")
            if 'TZ' in os.environ:
                orig_tz = os.environ['TZ']
            else:
                orig_tz = None
            os.environ['TZ'] = tz
            tzset()

            # now run the function, resetting the tz on exceptions
            try:
                return func(*args, **kwds)
            finally:
                if orig_tz is None:
                    del os.environ['TZ']
                else:
                    os.environ['TZ'] = orig_tz
                time.tzset()

        inner.__name__ = func.__name__
        inner.__doc__ = func.__doc__
        return inner
    return decorator


class Matcher(object):

    _partial_matches = ('msg', 'message')

    def matches(self, d, **kwargs):
        """
        Try to match a single dict with the supplied arguments.

        Keys whose values are strings and which are in self._partial_matches
        will be checked for partial (i.e. substring) matches. You can extend
        this scheme to (for example) do regular expression matching, etc.
        """
        result = True
        for k in kwargs:
            v = kwargs[k]
            dv = d.get(k)
            if not self.match_value(k, dv, v):
                result = False
                break
        return result

    def match_value(self, k, dv, v):
        """
        Try to match a single stored value (dv) with a supplied value (v).
        """
        if type(v) != type(dv):
            result = False
        elif type(dv) is not str or k not in self._partial_matches:
            result = (v == dv)
        else:
            result = dv.find(v) >= 0
        return result


@functools.total_ordering
class _LARGEST:
    """
    Object that is greater than anything (except itself).
    """
    def __eq__(self, other):
        return isinstance(other, _LARGEST)
    def __lt__(self, other):
        return False


@functools.total_ordering
class _SMALLEST:
    """
    Object that is less than anything (except itself).
    """
    def __eq__(self, other):
        return isinstance(other, _SMALLEST)
    def __gt__(self, other):
        return False


def check_disallow_instantiation(testcase, tp, *args, **kwds):
    """
    Check that given type cannot be instantiated using *args and **kwds.

    See bpo-43916: Add Py_TPFLAGS_DISALLOW_INSTANTIATION type flag.
    """
    mod = tp.__module__
    name = tp.__name__
    if mod != 'builtins':
        qualname = f"{mod}.{name}"
    else:
        qualname = f"{name}"
    msg = f"cannot create '{re.escape(qualname)}' instances"
    testcase.assertRaisesRegex(TypeError, msg, tp, *args, **kwds)
    testcase.assertRaisesRegex(TypeError, msg, tp.__new__, tp, *args, **kwds)


def get_recursion_depth():
    """Get the recursion depth of the caller function.

    In the __main__ module, at the module level, it should be 1.
    """
    try:
        import _testinternalcapi
        depth = _testinternalcapi.get_recursion_depth()
    except (ImportError, RecursionError, NotImplementedError) as exc:
        # sys._getframe() + frame.f_back implementation.
        try:
            depth = 0
            frame = sys._getframe()
            while frame is not None:
                depth += 1
                frame = frame.f_back
        finally:
            # Break any reference cycles.
            frame = None

    # Ignore get_recursion_depth() frame.
    return max(depth - 1, 1)


def get_recursion_available():
    """Get the number of available frames before RecursionError.

    It depends on the current recursion depth of the caller function and
    sys.getrecursionlimit().
    """
    limit = sys.getrecursionlimit()
    depth = get_recursion_depth()
    return limit - depth


@contextlib.contextmanager
def set_recursion_limit(limit):
    """Temporarily change the recursion limit."""
    original_limit = sys.getrecursionlimit()
    try:
        sys.setrecursionlimit(limit)
        yield
    finally:
        sys.setrecursionlimit(original_limit)


def infinite_recursion(max_depth=None):
    if max_depth is None:
        # Pick a number large enough to cause problems
        # but not take too long for code that can handle
        # very deep recursion.
        max_depth = 20_000
    elif max_depth < 3:
        raise ValueError(f"max_depth must be at least 3, got {max_depth}")
    depth = get_recursion_depth()
    depth = max(depth - 1, 1)  # Ignore infinite_recursion() frame.
    limit = depth + max_depth
    return set_recursion_limit(limit)
SHORT_TIMEOUT = 30.0


def walk_class_hierarchy(top, *, topdown=True):
    # This is based on the logic in os.walk().
    assert isinstance(top, type), repr(top)
    stack = [top]
    while stack:
        top = stack.pop()
        if isinstance(top, tuple):
            yield top
            continue

        subs = type(top).__subclasses__(top)
        if topdown:
            # Yield before subclass traversal if going top down.
            yield top, subs
            # Traverse into subclasses.
            for sub in reversed(subs):
                stack.append(sub)
        else:
            # Yield after subclass traversal if going bottom up.
            stack.append((top, subs))
            # Traverse into subclasses.
            for sub in reversed(subs):
                stack.append(sub)


def iter_builtin_types():
    # First try the explicit route.
    try:
        import _testinternalcapi
    except ImportError:
        _testinternalcapi = None
    if _testinternalcapi is not None:
        try:
            builtin_types = _testinternalcapi.get_static_builtin_types()
        except (AttributeError, NotImplementedError):
            pass
        else:
            yield from builtin_types
            return

    # Fall back to making a best-effort guess.
    if hasattr(object, '__flags__') and hasattr(type, '__subclasses__'):
        # Look for any type object with the Py_TPFLAGS_STATIC_BUILTIN flag set.
        import datetime
        seen = set()
        for cls, subs in walk_class_hierarchy(object):
            if cls in seen:
                continue
            seen.add(cls)
            if not (cls.__flags__ & _TPFLAGS_STATIC_BUILTIN):
                # Do not walk its subclasses.
                subs[:] = []
                continue
            yield cls
    else:
        # Fall back to a naive approach.
        seen = set()
        import builtins
        for obj in vars(builtins).values():
            if not isinstance(obj, type):
                continue
            cls = obj
            # XXX?
            if getattr(cls, '__module__', 'builtins') != 'builtins':
                continue
            if cls == ExceptionGroup:
                # It's a heap type.
                continue
            if cls in seen:
                continue
            seen.add(cls)
            yield cls


def identify_type_slot_wrappers():
    try:
        import _testinternalcapi
    except ImportError:
        _testinternalcapi = None
    if _testinternalcapi is not None:
        try:
            names = {n: None for n in _testinternalcapi.identify_type_slot_wrappers()}
        except (AttributeError, NotImplementedError):
            pass
        else:
            return list(names)
    raise NotImplementedError


def iter_slot_wrappers(cls):
    def is_slot_wrapper(name, value):
        if not isinstance(value, types.WrapperDescriptorType):
            assert not repr(value).startswith('<slot wrapper '), (cls, name, value)
            return False
        assert repr(value).startswith('<slot wrapper '), (cls, name, value)
        assert callable(value), (cls, name, value)
        assert name.startswith('__') and name.endswith('__'), (cls, name, value)
        return True

    try:
        attrs = identify_type_slot_wrappers()
    except NotImplementedError:
        attrs = None
    if attrs is not None:
        for attr in sorted(attrs):
            obj, base = find_name_in_mro(cls, attr, None)
            if obj is not None and is_slot_wrapper(attr, obj):
                yield attr, base is cls
        return

    # Fall back to a naive best-effort approach.

    try:
        ns = vars(cls)
    except TypeError:
        if not isinstance(cls, type):
            raise
        # Native kinds expose no namespace or wrapper descriptors yet.
        return
    unused = set(ns)
    for name in dir(cls):
        if name in ns:
            unused.remove(name)

        try:
            value = getattr(cls, name)
        except AttributeError:
            # It's as though it weren't in __dir__.
            assert name in ('__annotate__', '__annotations__', '__abstractmethods__'), (cls, name)
            if name in ns and is_slot_wrapper(name, ns[name]):
                unused.add(name)
            continue

        if not name.startswith('__') or not name.endswith('__'):
            assert not is_slot_wrapper(name, value), (cls, name, value)
        if not is_slot_wrapper(name, value):
            if name in ns:
                assert not is_slot_wrapper(name, ns[name]), (cls, name, value, ns[name])
        else:
            if name in ns:
                assert ns[name] is value, (cls, name, value, ns[name])
                yield name, True
            else:
                yield name, False

    for name in unused:
        value = ns[name]
        if is_slot_wrapper(cls, name, value):
            yield name, True


def run_no_yield_async_fn(async_fn, /, *args, **kwargs):
    coro = async_fn(*args, **kwargs)
    try:
        coro.send(None)
    except StopIteration as e:
        return e.value
    else:
        raise AssertionError("coroutine did not complete")
    finally:
        coro.close()


# TestLoader.discover is not implemented by the unittest stand-in yet.
class _PackageImportFailure(unittest.TestCase):
    def __init__(self, error):
        super().__init__('runTest')
        self.error = error

    def runTest(self):
        raise self.error


def _discover_package_tests(pkg_dir, loader, top_dir, pattern):
    import fnmatch
    from test.support.import_helper import DirsOnSysPath
    suite = unittest.TestSuite()
    top_dir = os.path.abspath(top_dir)
    prefix = top_dir.rstrip('/') + '/'

    def visit(directory):
        for filename in sorted(os.listdir(directory)):
            path = os.path.join(directory, filename)
            package = os.path.isdir(path)
            if package:
                if not os.path.isfile(os.path.join(path, '__init__.py')):
                    continue
            else:
                if (not filename.endswith('.py')
                        or not filename[:-3].isidentifier()
                        or not fnmatch.fnmatch(filename, pattern)):
                    continue
            absolute = os.path.abspath(path)
            if not absolute.startswith(prefix):
                raise ImportError('test package is outside the top level directory')
            name = absolute[len(prefix):].replace('/', '.')
            if not package:
                name = name[:-3]
            try:
                module = __import__(name, fromlist=['*'])
                # The stand-in loader cannot read a module namespace through
                # its host hook. Use Python's namespace and class predicates.
                tests = unittest.TestSuite()
                for class_name in dir(module):
                    candidate = getattr(module, class_name)
                    if (isinstance(candidate, type)
                            and issubclass(candidate, unittest.TestCase)
                            and candidate is not unittest.TestCase):
                        class_tests = loader.loadTestsFromTestCase(candidate)
                        for test in class_tests:
                            test._test_module = module.__name__
                        tests.addTests(class_tests)
                hook = getattr(module, 'load_tests', None)
                if hook is not None:
                    tests = hook(loader, tests, pattern)
                suite.addTests(tests)
                if package and hook is None:
                    visit(path)
            except Exception as error:
                suite.addTest(_PackageImportFailure(error))

    with DirsOnSysPath(top_dir):
        visit(pkg_dir)
    return suite

def iter_name_in_mro(cls, name):
    for base in cls.__mro__:
        ns = vars(base)
        if name in ns:
            yield ns[name], base

_mro_missing = object()
def find_name_in_mro(cls, name, default=_mro_missing):
    for res in iter_name_in_mro(cls, name):
        return res
    if default is not _mro_missing:
        return default, None
    raise AttributeError(name)

# The subprocess stand-in does not expose CPython's private flag helpers.
def optim_args_from_interpreter_flags():
    return ['-' + 'O' * sys.flags.optimize] if sys.flags.optimize else []

def args_from_interpreter_flags():
    import subprocess
    helper = getattr(subprocess, '_args_from_interpreter_flags', None)
    if helper is None:
        raise NotImplementedError('interpreter startup flags are not supported')
    return helper()

@contextlib.contextmanager
def check_no_resource_warning(testcase):
    from test.support.warnings_helper import check_no_warnings
    with check_no_warnings(testcase, category=ResourceWarning, force_gc=True):
        yield

LARGEST = _LARGEST()
SMALLEST = _SMALLEST()

@contextlib.contextmanager
def _run_with_locale(catstr, *locales):
    try:
        import locale
        category = getattr(locale, catstr)
        orig_locale = locale.setlocale(category)
    except AttributeError:
        # if the test author gives us an invalid category string
        raise
    except Exception:
        # cannot retrieve original locale, so do nothing
        locale = orig_locale = None
        if '' not in locales:
            raise unittest.SkipTest('no locales')
    else:
        for loc in locales:
            try:
                locale.setlocale(category, loc)
                break
            except (getattr(locale, "Error", ValueError), NotImplementedError):
                pass
        else:
            if '' not in locales:
                raise unittest.SkipTest(f'no locales {locales}')

    try:
        yield
    finally:
        if locale and orig_locale:
            locale.setlocale(category, orig_locale)


class _LocaleContextDecorator:
    # The embedded contextlib lacks ContextDecorator's recreation protocol.
    def __init__(self, catstr, locales):
        self.catstr = catstr
        self.locales = locales
        self.context = _run_with_locale(catstr, *locales)

    def __enter__(self):
        return self.context.__enter__()

    def __exit__(self, *exc):
        return self.context.__exit__(*exc)

    def __call__(self, func):
        @functools.wraps(func)
        def inner(*args, **kwargs):
            with _run_with_locale(self.catstr, *self.locales):
                return func(*args, **kwargs)
        return inner


def run_with_locale(catstr, *locales):
    return _LocaleContextDecorator(catstr, locales)


def no_rerun(reason):
    """Skip rerunning for a particular test.

    WARNING: Use this decorator with care; skipping rerunning makes it
    impossible to find reference leaks. Provide a clear reason for skipping the
    test using the 'reason' parameter.
    """
    import functools
    def deco(func):
        assert not isinstance(func, type), func
        _has_run = False
        @functools.wraps(func)
        def wrapper(self):
            nonlocal _has_run
            if _has_run:
                self.skipTest(reason)
            func(self)
            _has_run = True
        return wrapper
    return deco

# Build sanitizer detection for the runtime Rust compiler flags.
_build_flags = __host_info

def check_sanitizer(*, address=False, memory=False, ub=False, thread=False,
                    function=True):
    if not (address or memory or ub or thread):
        raise ValueError('At least one of address, memory, ub or thread must be True')
    flags = _build_flags('build')
    requested = ((address, 'address'), (memory, 'memory'),
                 (ub, 'undefined'), (thread, 'thread'), (function, 'function'))
    return any(enabled and ('sanitizer=' + name in flags or
                            '-fsanitize=' + name in flags)
               for enabled, name in requested)

# Repository root used to locate optional CPython source-tree resources.
REPO_ROOT = __file__.split("/langs/lib_python/modules/test/support/", 1)[0]

# Color controls mirror CPython's test-support contract.
@contextlib.contextmanager
def force_color(color):
    import _colorize
    from .os_helper import EnvironmentVarGuard
    with swap_attr(_colorize, "can_colorize", lambda *, file=None: color), EnvironmentVarGuard() as env:
        for name in ("FORCE_COLOR", "NO_COLOR", "PYTHON_COLORS"):
            env.unset(name)
        env.set("FORCE_COLOR" if color else "NO_COLOR", "1")
        yield

def force_colorized(func):
    @functools.wraps(func)
    def wrapper(*args, **kwargs):
        with force_color(True):
            return func(*args, **kwargs)
    return wrapper

def force_not_colorized(func):
    @functools.wraps(func)
    def wrapper(*args, **kwargs):
        with force_color(False):
            return func(*args, **kwargs)
    return wrapper

is_apple_mobile = sys.platform in {"ios", "tvos", "watchos"}

is_s390x = __host_info()[2] == "s390x"


def darwin_malloc_err_warning(test_name):
    """Assure user that loud errors generated by macOS libc's malloc are
    expected."""
    if sys.platform != 'darwin':
        return

    import shutil
    msg = ' NOTICE '
    detail = (f'{test_name} may generate "malloc can\'t allocate region"\n'
              'warnings on macOS systems. This behavior is known. Do not\n'
              'report a bug unless tests are also failing.\n'
              'See https://github.com/python/cpython/issues/85100')

    padding, _ = shutil.get_terminal_size()
    print(msg.center(padding, '-'))
    print(detail)
    print('-' * padding)



def open_urlresource(url, *args, **kwargs):
    if not is_resource_enabled('urlfetch'):
        raise unittest.SkipTest('resource urlfetch is not enabled')
    raise NotImplementedError('HTTP resource retrieval is not supported')

# CPython v3.14.8 test.support helper; PSF License.
_old_android_emulator = None
def setswitchinterval(interval):
    # Setting a very low gil interval on the Android emulator causes python
    # to hang (issue #26939).
    minimum_interval = 1e-4   # 100 us
    if is_android and interval < minimum_interval:
        global _old_android_emulator
        if _old_android_emulator is None:
            import platform
            av = platform.android_ver()
            _old_android_emulator = av.is_emulator and av.api_level < 24
        if _old_android_emulator:
            interval = minimum_interval
    return sys.setswitchinterval(interval)

# Return the complete C0 control-character set used by protocol parsers.
def control_characters_c0():
    return [chr(value) for value in range(32)] + ["\x7f"]
