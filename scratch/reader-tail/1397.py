# tests/python/test_with.py:493
if False:
    def testNestedExceptionAfterInnerStatement(self):
        mock_a = mock_contextmanager_generator()
        mock_b = mock_contextmanager_generator()
        def shouldThrow():
            with mock_a as self.foo:
                with mock_b as self.bar:
                    self.assertInWithManagerInvariants(mock_a)
                    self.assertInWithManagerInvariants(mock_b)
                    self.assertInWithGeneratorInvariants(self.foo)
                    self.assertInWithGeneratorInvariants(self.bar)
                self.raiseTestException()
        self.assertRaises(RuntimeError, shouldThrow)
        self.assertAfterWithManagerInvariantsWithError(mock_a)
        self.assertAfterWithManagerInvariantsNoError(mock_b)
        self.assertAfterWithGeneratorInvariantsWithError(self.foo)
        self.assertAfterWithGeneratorInvariantsNoError(self.bar)


print('read')
