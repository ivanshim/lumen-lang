# tests/python/test_set.py:74
if False:
    def test_contains(self):
        for c in self.letters:
            self.assertEqual(c in self.s, c in self.d)
        self.assertRaises(TypeError, self.s.__contains__, [[]])
        s = self.thetype([frozenset(self.letters)])
        self.assertIn(self.thetype(self.letters), s)


print('read')
