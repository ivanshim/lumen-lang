# tests/python/test_with.py:453
if False:
    def testMultipleResourcesInSingleStatement(self):
        cm_a = mock_contextmanager_generator()
        cm_b = mock_contextmanager_generator()
        mock_nested = MockNested(cm_a, cm_b)
        def shouldThrow():
            with mock_nested as (self.resource_a, self.resource_b):
                self.assertInWithManagerInvariants(cm_a)
                self.assertInWithManagerInvariants(cm_b)
                self.assertInWithManagerInvariants(mock_nested)
                self.assertInWithGeneratorInvariants(self.resource_a)
                self.assertInWithGeneratorInvariants(self.resource_b)
                self.raiseTestException()
        self.assertRaises(RuntimeError, shouldThrow)
        self.assertAfterWithManagerInvariantsWithError(cm_a)
        self.assertAfterWithManagerInvariantsWithError(cm_b)
        self.assertAfterWithManagerInvariantsWithError(mock_nested)
        self.assertAfterWithGeneratorInvariantsWithError(self.resource_a)
        self.assertAfterWithGeneratorInvariantsWithError(self.resource_b)


print('read')
