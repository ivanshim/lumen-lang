import unittest
from io import StringIO

class Subtests(unittest.TestCase):
    def test_numbers(self):
        self.addCleanup(print, 'cleaned')
        for number in [1, 2, 3]:
            with self.subTest(number=number):
                self.assertNotEqual(number, 2)
        print('continued')

stream = StringIO()
result = unittest.TextTestRunner(stream=stream).run(unittest.TestLoader().loadTestsFromTestCase(Subtests))
print(result.testsRun, len(result.failures), len(result.errors))
print('number=2' in stream.getvalue())
print('FAILED (failures=1)' in stream.getvalue())
