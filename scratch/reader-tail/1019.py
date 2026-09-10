# tests/python/test_grammar.py:244
if False:
    def test_ellipsis(self):
        x = ...
        self.assertTrue(x is Ellipsis)
        self.assertRaises(SyntaxError, eval, ".. .")


print('read')
