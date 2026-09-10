# tests/python/test_with.py:703
if False:
    def testWithExtendedTargets(self):
        with nullcontext(range(1, 5)) as (a, *b, c):
            self.assertEqual(a, 1)
            self.assertEqual(b, [2, 3])
            self.assertEqual(c, 4)



print('read')
