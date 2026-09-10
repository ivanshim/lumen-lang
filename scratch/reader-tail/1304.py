# tests/python/test_set.py:1813
if False:
    def test_constructor(self):
        for cons in (set, frozenset):
            for s in ("123", "", range(1000), ('do', 1.2), range(2000,2200,5)):
                for g in (G, I, Ig, S, L, R):
                    self.assertEqual(sorted(cons(g(s)), key=repr), sorted(g(s), key=repr))
                self.assertRaises(TypeError, cons , X(s))
                self.assertRaises(TypeError, cons , N(s))
                self.assertRaises(ZeroDivisionError, cons , E(s))


print('read')
