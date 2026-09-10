# tests/python/test_with.py:313
if False:
    def testInlineGeneratorBoundToExistingVariable(self):
        with mock_contextmanager_generator() as foo:
            self.assertInWithGeneratorInvariants(foo)
        self.assertAfterWithGeneratorInvariantsNoError(foo)


print('read')
