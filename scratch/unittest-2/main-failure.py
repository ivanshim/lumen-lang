# A failing default main run terminates with a nonzero status.
import unittest

class MainFailure(unittest.TestCase):
    def test_failure(self):
        self.fail('expected failure')

unittest.main()
print('unreachable after failed main')
