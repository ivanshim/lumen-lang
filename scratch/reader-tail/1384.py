# tests/python/test_with.py:331
if False:
    def testNestedSingleStatements(self):
        mock_a = mock_contextmanager_generator()
        with mock_a as foo:
            mock_b = mock_contextmanager_generator()
            with mock_b as bar:
                self.assertInWithManagerInvariants(mock_a)
                self.assertInWithManagerInvariants(mock_b)
                self.assertInWithGeneratorInvariants(foo)
                self.assertInWithGeneratorInvariants(bar)
            self.assertAfterWithManagerInvariantsNoError(mock_b)
            self.assertAfterWithGeneratorInvariantsNoError(bar)
            self.assertInWithManagerInvariants(mock_a)
            self.assertInWithGeneratorInvariants(foo)
        self.assertAfterWithManagerInvariantsNoError(mock_a)
        self.assertAfterWithGeneratorInvariantsNoError(foo)



print('read')
