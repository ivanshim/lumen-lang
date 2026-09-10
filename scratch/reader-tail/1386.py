# tests/python/test_with.py:354
if False:
    def testSingleArgBoundToNonTuple(self):
        m = mock_contextmanager_generator()
        # This will bind all the arguments to nested() into a single list
        # assigned to foo.
        with Nested(m) as foo:
            self.assertInWithManagerInvariants(m)
        self.assertAfterWithManagerInvariantsNoError(m)


print('read')
