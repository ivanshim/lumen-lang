# tests/python/test_set.py:1547
if False:
    def test_sym_difference(self):
        self.assertRaises(TypeError, lambda: self.set ^ self.other)
        self.assertRaises(TypeError, lambda: self.other ^ self.set)
        if self.otherIsIterable:
            self.set.symmetric_difference(self.other)
        else:
            self.assertRaises(TypeError, self.set.symmetric_difference, self.other)


print('read')
