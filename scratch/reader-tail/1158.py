# tests/python/test_set.py:872
if False:
    def test_singleton_empty_frozenset(self):
        Frozenset = self.thetype
        f = frozenset()
        F = Frozenset()
        efs = [Frozenset(), Frozenset([]), Frozenset(()), Frozenset(''),
               Frozenset(), Frozenset([]), Frozenset(()), Frozenset(''),
               Frozenset(range(0)), Frozenset(Frozenset()),
               Frozenset(frozenset()), f, F, Frozenset(f), Frozenset(F)]
        # All empty frozenset subclass instances should have different ids
        self.assertEqual(len(set(map(id, efs))), len(efs))



print('read')
