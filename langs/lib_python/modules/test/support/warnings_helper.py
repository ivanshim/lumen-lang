# These warning helpers are stubs, since no warnings are emitted.
from warnings import catch_warnings

def check_warnings(*filters, quiet=True):
    return catch_warnings(record=True)

def ignore_warnings(*args, **kwargs):
    return _identity

def _identity(function):
    return function

def ignore_fork_in_thread_deprecation_warnings(function):
    return function
