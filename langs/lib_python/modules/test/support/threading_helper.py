
# Derived from CPython v3.14.8 Lib/test/support; PSF License.
import contextlib
import functools
import threading
import time
import sys
import unittest
from test import support

can_start_thread = False

def reap_threads(func):
    """Use this function when threads are being used.  This will
    ensure that the threads are cleaned up even when the test fails.
    """
    @functools.wraps(func)
    def decorator(*args):
        key = threading_setup()
        try:
            return func(*args)
        finally:
            threading_cleanup(*key)
    return decorator


def join_thread(thread, timeout=None):
    """Join a thread. Raise an AssertionError if the thread is still alive
    after timeout seconds.
    """
    if timeout is None:
        timeout = support.SHORT_TIMEOUT
    thread.join(timeout)
    if thread.is_alive():
        msg = f"failed to join the thread in {timeout:.1f} seconds"
        raise AssertionError(msg)


@contextlib.contextmanager
def start_threads(threads, unlock=None):
    try:
        import faulthandler
    except ImportError:
        # It isn't supported on subinterpreters yet.
        faulthandler = None
    threads = list(threads)
    started = []
    try:
        try:
            for t in threads:
                t.start()
                started.append(t)
        except:
            if support.verbose:
                print("Can't start %d threads, only %d threads started" %
                      (len(threads), len(started)))
            raise
        yield
    finally:
        try:
            if unlock:
                unlock()
            endtime = time.monotonic()
            for timeout in range(1, 16):
                endtime += 60
                for t in started:
                    t.join(max(endtime - time.monotonic(), 0.01))
                started = [t for t in started if t.is_alive()]
                if not started:
                    break
                if support.verbose:
                    print('Unable to join %d threads during a period of '
                          '%d minutes' % (len(started), timeout))
        finally:
            started = [t for t in started if t.is_alive()]
            if started:
                if faulthandler is not None:
                    faulthandler.dump_traceback(sys.stdout)
                raise AssertionError('Unable to join %d threads' % len(started))


class catch_threading_exception:
    """
    Context manager catching threading.Thread exception using
    threading.excepthook.

    Attributes set when an exception is caught:

    * exc_type
    * exc_value
    * exc_traceback
    * thread

    See threading.excepthook() documentation for these attributes.

    These attributes are deleted at the context manager exit.

    Usage:

        with threading_helper.catch_threading_exception() as cm:
            # code spawning a thread which raises an exception
            ...

            # check the thread exception, use cm attributes:
            # exc_type, exc_value, exc_traceback, thread
            ...

        # exc_type, exc_value, exc_traceback, thread attributes of cm no longer
        # exists at this point
        # (to avoid reference cycles)
    """

    def __init__(self):
        self.exc_type = None
        self.exc_value = None
        self.exc_traceback = None
        self.thread = None
        self._old_hook = None

    def _hook(self, args):
        self.exc_type = args.exc_type
        self.exc_value = args.exc_value
        self.exc_traceback = args.exc_traceback
        self.thread = args.thread

    def __enter__(self):
        self._old_hook = threading.excepthook
        threading.excepthook = self._hook
        return self

    def __exit__(self, *exc_info):
        threading.excepthook = self._old_hook
        del self.exc_type
        del self.exc_value
        del self.exc_traceback
        del self.thread


def requires_working_threading(*, module=False):
    """Skip tests or modules that require working threading.

    Can be used as a function/class decorator or to skip an entire module.
    """
    msg = "requires threading support"
    if module:
        if not can_start_thread:
            raise unittest.SkipTest(msg)
    else:
        return unittest.skipUnless(can_start_thread, msg)


def run_concurrently(worker_func, nthreads=None, args=(), kwargs={}):
    """
    Run the worker function(s) concurrently in multiple threads.

    If `worker_func` is a single callable, it is used for all threads.
    If it is a list of callables, each callable is used for one thread.
    """
    from collections.abc import Iterable

    if nthreads is None:
        nthreads = len(worker_func)
    if not isinstance(worker_func, Iterable):
        worker_func = [worker_func] * nthreads
    assert len(worker_func) == nthreads

    barrier = threading.Barrier(nthreads)

    def wrapper_func(func, *args, **kwargs):
        # Wait for all threads to reach this point before proceeding.
        barrier.wait()
        func(*args, **kwargs)

    with catch_threading_exception() as cm:
        workers = [
            threading.Thread(target=wrapper_func, args=(func, *args), kwargs=kwargs)
            for func in worker_func
        ]
        with start_threads(workers):
            pass

        # If a worker thread raises an exception, re-raise it.
        if cm.exc_value is not None:
            raise cm.exc_value


# There is one live thread and no native _thread counter or dangling registry.
def threading_setup():
    return threading.active_count() - 1, 0

def threading_cleanup(*original_values):
    if threading.active_count() - 1 > original_values[0]:
        raise AssertionError('threads remain after cleanup')

@contextlib.contextmanager
def wait_threads_exit(timeout=None):
    original = threading_setup()
    try:
        yield
    finally:
        threading_cleanup(*original)
