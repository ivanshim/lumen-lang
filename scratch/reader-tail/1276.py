# tests/python/test_set.py:1650
if False:
        def test_deep_copy(self):
            dup = copy.deepcopy(self.set)
            ##print type(dup), repr(dup)
            dup_list = sorted(dup, key=repr)
            set_list = sorted(self.set, key=repr)
            self.assertEqual(len(dup_list), len(set_list))
            for i in range(len(dup_list)):
                self.assertEqual(dup_list[i], set_list[i])

    #------------------------------------------------------------------------------


print('read')
