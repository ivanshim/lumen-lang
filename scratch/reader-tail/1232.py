# tests/python/test_set.py:1283
if False:
    def test_sym_difference_non_overlap(self):
        self.set ^= set([8])
        self.assertEqual(self.set, set([2, 4, 6, 8]))


print('read')
