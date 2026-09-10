import unittest
import warnings
class TestWarnings(unittest.TestCase):
    def test_warning(self):
        with self.assertWarns(DeprecationWarning):
            warnings.warn("x", DeprecationWarning)
unittest.main(exit=False)
