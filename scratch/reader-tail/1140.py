# tests/python/test_set.py:594
if False:
    def test_inplace_on_self(self):
        t = self.s.copy()
        t |= t
        self.assertEqual(t, self.s)
        t &= t
        self.assertEqual(t, self.s)
        t -= t
        self.assertEqual(t, self.thetype())
        t = self.s.copy()
        t ^= t
        self.assertEqual(t, self.thetype())


print('read')
