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
        self.function = function
        return self.call

    def call(self, testcase):
        size = self.size if real_max_memuse else 5147
        if (real_max_memuse or not self.dry_run) and real_max_memuse < size * self.memuse:
            raise unittest.SkipTest('not enough memory: %.1fG minimum needed' %
                                    (self.size * self.memuse / (1024 ** 3)))
        return self.function(testcase, size)

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
TEST_HOME_DIR = os.path.join(os.path.dirname(os.path.dirname(os.path.dirname(os.path.dirname(os.path.dirname(os.path.dirname(__file__)))))), 'tests', 'python')
_header = 'nP'
_align = '0n'
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
    path = [TEST_HOME_DIR] + sys.path
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
