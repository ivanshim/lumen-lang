# tests/python/test_set.py:422
if False:
    def test_add(self):
        self.s.add('Q')
        self.assertIn('Q', self.s)
        dup = self.s.copy()
        self.s.add('Q')
        self.assertEqual(self.s, dup)
        self.assertRaises(TypeError, self.s.add, [])


print('read')
