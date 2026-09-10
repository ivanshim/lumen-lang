import unittest

def cleanup(message):
    print(message)

class LifecycleTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        print('class setup')

    def test_cases(self):
        self.addCleanup(cleanup, 'cleanup')
        for number in [1, 2, 3]:
            with self.subTest(number=number):
                self.assertNotEqual(number, 2)

    @unittest.skipIf(True, 'not today')
    def test_skip(self):
        self.fail('must be skipped')

    @unittest.expectedFailure
    def test_expected(self):
        self.fail('known failure')

program = unittest.main(exit=False)
result = program.result
print(result.testsRun, len(result.failures), len(result.errors), len(result.skipped), len(result.expectedFailures))
