# tests/python/test_set.py:1523
if False:
    def test_intersection(self):
        self.assertRaises(TypeError, lambda: self.set & self.other)
        self.assertRaises(TypeError, lambda: self.other & self.set)
        if self.otherIsIterable:
            self.set.intersection(self.other)
        else:
            self.assertRaises(TypeError, self.set.intersection, self.other)


print('read')
