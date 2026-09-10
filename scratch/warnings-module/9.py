import unittest
import warnings
case = unittest.TestCase()
with case.assertWarnsRegex(UserWarning, '^he.*o$') as capture:
    warnings.warn('hello')
print(str(capture.warning), capture.lineno)
print(len(capture.warnings), capture.warnings[0].category.__name__)
