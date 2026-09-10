# tests/python/test_set.py:172
if False:
    def test_sub(self):
        i = self.s.difference(self.otherword)
        self.assertEqual(self.s - set(self.otherword), i)
        self.assertEqual(self.s - frozenset(self.otherword), i)
        try:
            self.s - self.otherword
        except TypeError:
            pass
        else:
            self.fail("s-t did not screen-out general iterables")


print('read')
