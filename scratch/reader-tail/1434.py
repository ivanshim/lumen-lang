# tests/python/test_decorators.py:148
if False:
    def test_memoize(self):
        counts = {}

        @memoize
        @countcalls(counts)
        def double(x):
            return x * 2
        self.assertEqual(double.__name__, 'double')

        self.assertEqual(counts, dict(double=0))

        # Only the first call with a given argument bumps the call count:
        #
        self.assertEqual(double(2), 4)
        self.assertEqual(counts['double'], 1)
        self.assertEqual(double(2), 4)
        self.assertEqual(counts['double'], 1)
        self.assertEqual(double(3), 6)
        self.assertEqual(counts['double'], 2)

        # Unhashable arguments do not get memoized:
        #
        self.assertEqual(double([10]), [10, 10])
        self.assertEqual(counts['double'], 3)
        self.assertEqual(double([10]), [10, 10])
        self.assertEqual(counts['double'], 4)


print('read')
