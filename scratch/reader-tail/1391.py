# tests/python/test_with.py:400
if False:
    def testMultipleArgBound(self):
        mock_nested = MockNested(mock_contextmanager_generator(),
            mock_contextmanager_generator(), mock_contextmanager_generator())
        with mock_nested as (m, n, o):
            self.assertInWithGeneratorInvariants(m)
            self.assertInWithGeneratorInvariants(n)
            self.assertInWithGeneratorInvariants(o)
            self.assertInWithManagerInvariants(mock_nested)
        self.assertAfterWithGeneratorInvariantsNoError(m)
        self.assertAfterWithGeneratorInvariantsNoError(n)
        self.assertAfterWithGeneratorInvariantsNoError(o)
        self.assertAfterWithManagerInvariantsNoError(mock_nested)



print('read')
