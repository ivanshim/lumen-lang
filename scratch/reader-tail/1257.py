# tests/python/test_set.py:1493
if False:
    def test_update(self):
        if self.otherIsIterable:
            self.set.update(self.other)
        else:
            self.assertRaises(TypeError, self.set.update, self.other)


print('read')
