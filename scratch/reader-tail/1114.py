# tests/python/test_set.py:326
if False:
    def test_cyclical_repr(self):
        w = ReprWrapper()
        s = self.thetype([w])
        w.value = s
        if self.thetype == set:
            self.assertEqual(repr(s), '{set(...)}')
        else:
            name = repr(s).partition('(')[0]    # strip class name
            self.assertEqual(repr(s), '%s({%s(...)})' % (name, name))


print('read')
