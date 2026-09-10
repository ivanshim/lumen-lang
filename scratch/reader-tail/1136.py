# tests/python/test_set.py:533
if False:
    def test_difference_update(self):
        retval = self.s.difference_update(self.otherword)
        self.assertEqual(retval, None)
        for c in (self.word + self.otherword):
            if c in self.word and c not in self.otherword:
                self.assertIn(c, self.s)
            else:
                self.assertNotIn(c, self.s)
        self.assertRaises(PassThru, self.s.difference_update, check_pass_thru())
        self.assertRaises(TypeError, self.s.difference_update, [[]])
        self.assertRaises(TypeError, self.s.symmetric_difference_update, [[]])
        for p, q in (('cdc', 'ab'), ('efgfe', 'abc'), ('ccb', 'a'), ('ef', 'abc')):
            for C in set, frozenset, dict.fromkeys, str, list, tuple:
                s = self.thetype('abcba')
                self.assertEqual(s.difference_update(C(p)), None)
                self.assertEqual(s, set(q))

                s = self.thetype('abcdefghih')
                s.difference_update()
                self.assertEqual(s, self.thetype('abcdefghih'))

                s = self.thetype('abcdefghih')
                s.difference_update(C('aba'))
                self.assertEqual(s, self.thetype('cdefghih'))

                s = self.thetype('abcdefghih')
                s.difference_update(C('cdc'), C('aba'))
                self.assertEqual(s, self.thetype('efghih'))


print('read')
