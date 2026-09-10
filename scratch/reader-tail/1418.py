# tests/python/test_with.py:803
if False:
    def testEnterReturnsTuple(self):
        with self.Dummy(value=(1,2)) as (a1, a2), \
             self.Dummy(value=(10, 20)) as (b1, b2):
            self.assertEqual(1, a1)
            self.assertEqual(2, a2)
            self.assertEqual(10, b1)
            self.assertEqual(20, b2)


print('read')
