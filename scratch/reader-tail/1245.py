# tests/python/test_set.py:1346
if False:
    def test_remove_until_empty(self):
        expected_len = len(self.set)
        for v in self.values:
            self.set.remove(v)
            expected_len -= 1
            self.assertEqual(len(self.set), expected_len)


print('read')
