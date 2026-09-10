# tests/python/test_with.py:362
if False:
    def testSingleArgBoundToSingleElementParenthesizedList(self):
        m = mock_contextmanager_generator()
        # This will bind all the arguments to nested() into a single list
        # assigned to foo.
        with Nested(m) as (foo):
            self.assertInWithManagerInvariants(m)
        self.assertAfterWithManagerInvariantsNoError(m)


print('read')
