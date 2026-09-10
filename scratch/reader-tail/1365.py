# tests/python/test_with.py:208
if False:
    def testAssignmentToTupleOnlyContainingNoneError(self):
        self.assertRaisesSyntaxError('with mock as None,:\n  pass')
        self.assertRaisesSyntaxError(
            'with mock as (None,):\n'
            '  pass')


print('read')
