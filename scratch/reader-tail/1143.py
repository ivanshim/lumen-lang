# tests/python/test_set.py:650
if False:
    def test_set_membership(self):
        myfrozenset = frozenset(range(3))
        myset = {myfrozenset, "abc", 1}
        self.assertIn(set(range(3)), myset)
        self.assertNotIn(set(range(1)), myset)
        myset.discard(set(range(3)))
        self.assertEqual(myset, {"abc", 1})
        self.assertRaises(KeyError, myset.remove, set(range(1)))
        self.assertRaises(KeyError, myset.remove, set(range(3)))


print('read')
