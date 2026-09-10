# tests/python/test_set.py:383
if False:
    def test_constructor_identity(self):
        s = self.thetype(range(3))
        t = self.thetype(s)
        self.assertNotEqual(id(s), id(t))


print('read')
