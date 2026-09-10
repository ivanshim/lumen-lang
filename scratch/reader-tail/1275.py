# tests/python/test_set.py:1642
if False:
    def test_copy(self):
        dup = self.set.copy()
        dup_list = sorted(dup, key=repr)
        set_list = sorted(self.set, key=repr)
        self.assertEqual(len(dup_list), len(set_list))
        for i in range(len(dup_list)):
            self.assertTrue(dup_list[i] is set_list[i])


print('read')
