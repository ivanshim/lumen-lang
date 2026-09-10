# tests/python/test_with.py:214
if False:
    def testAssignmentToTupleContainingNoneError(self):
        self.assertRaisesSyntaxError(
            'with mock as (foo, None, bar):\n'
            '  pass')


print('read')
