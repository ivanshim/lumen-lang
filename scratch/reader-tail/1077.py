# tests/python/test_grammar.py:1959
if False:
    def test_paren_evaluation(self):
        self.assertEqual(16 // (4 // 2), 8)
        self.assertEqual((16 // 4) // 2, 2)
        self.assertEqual(16 // 4 // 2, 2)
        x = 2
        y = 3
        self.assertTrue(False is (x is y))
        self.assertFalse((False is x) is y)
        self.assertFalse(False is x is y)


print('read')
