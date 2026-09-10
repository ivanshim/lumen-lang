# tests/python/test_set.py:606
if False:
    def test_weakref(self):
        s = self.thetype('gallahad')
        p = weakref.proxy(s)
        self.assertEqual(str(p), str(s))
        s = None
        support.gc_collect()  # For PyPy or other GCs.
        self.assertRaises(ReferenceError, str, p)


print('read')
