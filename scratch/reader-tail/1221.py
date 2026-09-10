# tests/python/test_set.py:1239
if False:
    def test_union_overlap(self):
        self.set |= set([3, 4, 5])
        self.assertEqual(self.set, set([2, 3, 4, 5, 6]))


print('read')
