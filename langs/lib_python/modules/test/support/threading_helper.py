# No worker threads are provided by this runtime.
import functools
import unittest

def requires_working_threading(module=False):
    if module:
        raise unittest.SkipTest('threads are not supported')
    return unittest.skip('threads are not supported')

def start_threads(threads, unlock=None):
    raise 'NotImplementedError: threads are not supported'


def reap_threads(func):
    # No worker threads can be started by this runtime, so there are none
    # to reap; the decorator keeps its shape.
    @functools.wraps(func)
    def decorator(*args):
        return func(*args)
    return decorator
