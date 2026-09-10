# tests/python/test_set.py:689
if False:
    def test_hash_collision_remove_add(self):
        self.maxDiff = None
        # There should be enough space, so all elements with unique hash
        # will be placed in corresponding cells without collision.
        n = 64
        elems = [CustomHash(h) for h in range(n)]
        # Elements with hash collision.
        a = CustomHash(n)
        b = CustomHash(n)
        elems += [a, b]
        s = self.thetype(elems)
        self.assertEqual(len(s), len(elems), s)
        s.remove(a)
        # "a" has been replaced with a dummy.
        del elems[n]
        self.assertEqual(len(s), len(elems), s)
        self.assertEqual(s, set(elems))
        s.add(b)
        # "b" should not replace the dummy.
        self.assertEqual(len(s), len(elems), s)
        self.assertEqual(s, set(elems))



print('read')
