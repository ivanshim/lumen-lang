import unittest

class RaisesTest(unittest.TestCase):
    def test_context(self):
        with self.assertRaises(self.failureException) as caught:
            self.fail('caught')
        print(caught.exception.args[0])

unittest.main(verbosity=2, exit=False)
