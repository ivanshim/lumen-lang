import unittest
from test import support

class Resource:
    def __getattr__(self, name):
        raise AttributeError(name)
    def __del__(self):
        raise OSError('closed resource')

with support.catch_unraisable_exception() as catcher:
    with unittest.TestCase().assertRaises(AttributeError):
        Resource().missing
    assert catcher.unraisable.exc_type is OSError
    assert str(catcher.unraisable.exc_value) == 'closed resource'
print('completed exception frames release their resource')
