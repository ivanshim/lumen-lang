# tests/python/test_set.py:995
if False:
    def test_pickling(self):
        for proto in range(pickle.HIGHEST_PROTOCOL + 1):
            p = pickle.dumps(self.set, proto)
            copy = pickle.loads(p)
            self.assertEqual(self.set, copy,
                             "%s != %s" % (self.set, copy))


print('read')
