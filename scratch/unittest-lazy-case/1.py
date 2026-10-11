import sys
import unittest
assert 'unittest.case' not in sys.modules
assert 'case' in dir(unittest)
assert not hasattr(unittest, 'missing_attribute')
case = unittest.case
from unittest import case as imported_case
import unittest.case
assert case is imported_case
assert case is unittest.case
assert case is sys.modules['unittest.case']
import warnings
with case.TestCase().assertWarns(UserWarning):
    warnings.warn('lazy case still provides warning contexts', UserWarning)
print('unittest case loads on demand and retains module identity and warning contexts')
