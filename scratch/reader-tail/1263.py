# tests/python/test_set.py:1539
if False:
    def test_sym_difference_update(self):
        if self.otherIsIterable:
            self.set.symmetric_difference_update(self.other)
        else:
            self.assertRaises(TypeError,
                              self.set.symmetric_difference_update,
                              self.other)


print('read')
