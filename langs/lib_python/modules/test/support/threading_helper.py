# No worker threads are provided by this runtime.
import unittest

def requires_working_threading(module=False):
    if module:
        raise unittest.SkipTest('threads are not supported')
    return unittest.skip('threads are not supported')

def start_threads(threads, unlock=None):
    raise 'NotImplementedError: threads are not supported'


def reap_threads(func):
    # No thread can be started in this runtime, so there are none to join.
    return func
