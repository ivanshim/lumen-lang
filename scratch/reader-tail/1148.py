# tests/python/test_set.py:760
if False:
    def test_constructor_identity(self):
        s = self.thetype(range(3))
        t = self.thetype(s)
        self.assertEqual(id(s), id(t))


print('read')
