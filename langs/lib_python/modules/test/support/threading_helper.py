# No worker threads are provided by this runtime.
import unittest

def requires_working_threading(module=False):
    if module:
        raise unittest.SkipTest('threads are not supported')
    return unittest.skip('threads are not supported')

def start_threads(threads, unlock=None):
    raise 'NotImplementedError: threads are not supported'

# Helpers from CPython 3b564385e4c9 Lib/test/support/threading_helper.py; PSF License.
import _thread
import functools
import threading
from test import support

def threading_setup():
    return _thread._count(), len(threading._dangling)


def threading_cleanup(*original_values):
    orig_count, orig_ndangling = original_values

    timeout = 1.0
    for _ in support.sleeping_retry(timeout, error=False):
        # Copy the thread list to get a consistent output. threading._dangling
        # is a WeakSet, its value changes when it's read.
        dangling_threads = list(threading._dangling)
        count = _thread._count()

        if count <= orig_count:
            return

    # Timeout!
    support.environment_altered = True
    support.print_warning(
        f"threading_cleanup() failed to clean up threads "
        f"in {timeout:.1f} seconds\n"
        f"  before: thread count={orig_count}, dangling={orig_ndangling}\n"
        f"  after: thread count={count}, dangling={len(dangling_threads)}")
    for thread in dangling_threads:
        support.print_warning(f"Dangling thread: {thread!r}")

    # The warning happens when a test spawns threads and some of these threads
    # are still running after the test completes. To fix this warning, join
    # threads explicitly to wait until they complete.
    #
    # To make the warning more likely, reduce the timeout.


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
