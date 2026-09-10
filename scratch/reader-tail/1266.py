# tests/python/test_set.py:1563
if False:
    def test_difference_update(self):
        if self.otherIsIterable:
            self.set.difference_update(self.other)
        else:
            self.assertRaises(TypeError,
                              self.set.difference_update,
                              self.other)


print('read')
