# Keyword arguments reach test helpers; ordinary errors do not stop the suite.
import unittest
from io import StringIO

def total(first, *, second):
    return first + second

class ErrorsRecorded(unittest.TestCase):
    def test_error(self):
        return 1 / 0

    def test_keywords(self):
        self.assertEqual(total(2, second=3), 5)
        self.assertRaises(self.failureException, self.assertEqual, 1, 2, msg='keyword message')
        print('continued after error')

stream = StringIO()
result = unittest.TextTestRunner(stream=stream).run(unittest.TestLoader().loadTestsFromTestCase(ErrorsRecorded))
print(result.testsRun, len(result.failures), len(result.errors), result.wasSuccessful())
print('ERROR: test_error' in stream.getvalue())
print('FAILED (errors=1)' in stream.getvalue())
