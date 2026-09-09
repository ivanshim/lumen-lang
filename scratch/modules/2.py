import unittest

class Example(unittest.TestCase):
    def test_equal(self):
        self.assertEqual(2 + 2, 4)

    def test_truth(self):
        self.assertTrue(True)

    def test_wrong(self):
        self.assertEqual(1, 2)

unittest.main()
