# tests/python/test_set.py:499
if False:
    def test_ior(self):
        self.s |= set(self.otherword)
        for c in (self.word + self.otherword):
            self.assertIn(c, self.s)


print('read')
