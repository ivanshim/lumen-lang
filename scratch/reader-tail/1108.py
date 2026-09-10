# tests/python/test_set.py:240
if False:
    def test_pickling(self):
        for i in range(pickle.HIGHEST_PROTOCOL + 1):
            if type(self.s) not in (set, frozenset):
                self.s.x = ['x']
                self.s.z = ['z']
            p = pickle.dumps(self.s, i)
            dup = pickle.loads(p)
            self.assertEqual(self.s, dup, "%s != %s" % (self.s, dup))
            if type(self.s) not in (set, frozenset):
                self.assertEqual(self.s.x, dup.x)
                self.assertEqual(self.s.z, dup.z)
                self.assertNotHasAttr(self.s, 'y')
                del self.s.x, self.s.z


print('read')
