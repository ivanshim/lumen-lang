# tests/python/test_set.py:1822
if False:
    def test_inline_methods(self):
        s = set('november')
        for data in ("123", "", range(1000), ('do', 1.2), range(2000,2200,5), 'december'):
            for meth in (s.union, s.intersection, s.difference, s.symmetric_difference, s.isdisjoint):
                for g in (G, I, Ig, L, R):
                    expected = meth(data)
                    actual = meth(g(data))
                    if isinstance(expected, bool):
                        self.assertEqual(actual, expected)
                    else:
                        self.assertEqual(sorted(actual, key=repr), sorted(expected, key=repr))
                self.assertRaises(TypeError, meth, X(s))
                self.assertRaises(TypeError, meth, N(s))
                self.assertRaises(ZeroDivisionError, meth, E(s))


print('read')
