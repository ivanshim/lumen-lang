# tests/python/test_set.py:416
if False:
    def test_copy(self):
        dup = self.s.copy()
        self.assertEqual(self.s, dup)
        self.assertNotEqual(id(self.s), id(dup))
        self.assertEqual(type(dup), self.basetype)


print('read')
