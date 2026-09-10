# tests/python/test_set.py:1326
if False:
    def test_add_until_full(self):
        tmp = set()
        expected_len = 0
        for v in self.values:
            tmp.add(v)
            expected_len += 1
            self.assertEqual(len(tmp), expected_len)
        self.assertEqual(tmp, self.set)


print('read')
