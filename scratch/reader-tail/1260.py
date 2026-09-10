# tests/python/test_set.py:1515
if False:
    def test_intersection_update(self):
        if self.otherIsIterable:
            self.set.intersection_update(self.other)
        else:
            self.assertRaises(TypeError,
                              self.set.intersection_update,
                              self.other)


print('read')
