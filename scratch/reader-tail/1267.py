# tests/python/test_set.py:1571
if False:
        def test_difference(self):
            self.assertRaises(TypeError, lambda: self.set - self.other)
            self.assertRaises(TypeError, lambda: self.other - self.set)
            if self.otherIsIterable:
                self.set.difference(self.other)
            else:
                self.assertRaises(TypeError, self.set.difference, self.other)

    #------------------------------------------------------------------------------


print('read')
