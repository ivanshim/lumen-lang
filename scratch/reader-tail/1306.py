# tests/python/test_set.py:1837
if False:
    def test_inplace_methods(self):
        for data in ("123", "", range(1000), ('do', 1.2), range(2000,2200,5), 'december'):
            for methname in ('update', 'intersection_update',
                             'difference_update', 'symmetric_difference_update'):
                for g in (G, I, Ig, S, L, R):
                    s = set('january')
                    t = s.copy()
                    getattr(s, methname)(list(g(data)))
                    getattr(t, methname)(g(data))
                    self.assertEqual(sorted(s, key=repr), sorted(t, key=repr))

                self.assertRaises(TypeError, getattr(set('january'), methname), X(data))
                self.assertRaises(TypeError, getattr(set('january'), methname), N(data))
                self.assertRaises(ZeroDivisionError, getattr(set('january'), methname), E(data))


print('read')
