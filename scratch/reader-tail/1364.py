# tests/python/test_with.py:202
if False:
    def testAssignmentToNoneError(self):
        self.assertRaisesSyntaxError('with mock as None:\n  pass')
        self.assertRaisesSyntaxError(
            'with mock as (None):\n'
            '  pass')


print('read')
