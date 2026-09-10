# tests/python/test_with.py:385
if False:
    def testMultipleArgUnbound(self):
        m = mock_contextmanager_generator()
        n = mock_contextmanager_generator()
        o = mock_contextmanager_generator()
        mock_nested = MockNested(m, n, o)
        with mock_nested:
            self.assertInWithManagerInvariants(m)
            self.assertInWithManagerInvariants(n)
            self.assertInWithManagerInvariants(o)
            self.assertInWithManagerInvariants(mock_nested)
        self.assertAfterWithManagerInvariantsNoError(m)
        self.assertAfterWithManagerInvariantsNoError(n)
        self.assertAfterWithManagerInvariantsNoError(o)
        self.assertAfterWithManagerInvariantsNoError(mock_nested)


print('read')
