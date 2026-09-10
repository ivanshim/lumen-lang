# tests/python/test_set.py:1706
if False:
    def test_commutativity(self):
        a, b = self.a, self.b
        self.assertEqual(a&b, b&a)
        self.assertEqual(a|b, b|a)
        self.assertEqual(a^b, b^a)
        if a != b:
            self.assertNotEqual(a-b, b-a)


print('read')
