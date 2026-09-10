# tests/python/test_decorators.py:141
if False:
    def test_dbcheck(self):
        @dbcheck('args[1] is not None')
        def f(a, b):
            return a + b
        self.assertEqual(f(1, 2), 3)
        self.assertRaises(DbcheckError, f, 1, None)


print('read')
