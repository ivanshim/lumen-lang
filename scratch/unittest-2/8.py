import unittest

def cleanup(value):
    print(value)

class Lifecycle(unittest.TestCase):
    def setUp(self):
        print('setup')
        self.addCleanup(cleanup, value='cleanup')

    def test_ok(self):
        print('body')

    def tearDown(self):
        print('teardown')

result = unittest.TestResult()
Lifecycle('test_ok').run(result)
print(result.testsRun, result.wasSuccessful())
