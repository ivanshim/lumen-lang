import unittest
import warnings
case = unittest.TestCase()
with case.assertWarnsRegex(UserWarning, '^he.*o$') as capture:
    warnings.warn('hello')
print(str(capture.warning), capture.lineno)
