# tests/python/test_set.py:586
if False:
    def test_ixor(self):
        self.s ^= set(self.otherword)
        for c in (self.word + self.otherword):
            if (c in self.word) ^ (c in self.otherword):
                self.assertIn(c, self.s)
            else:
                self.assertNotIn(c, self.s)


print('read')
