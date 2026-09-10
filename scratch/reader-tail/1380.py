# tests/python/test_with.py:307
if False:
    def testInlineGeneratorBoundSyntax(self):
        with mock_contextmanager_generator() as foo:
            self.assertInWithGeneratorInvariants(foo)
        # FIXME: In the future, we'll try to keep the bound names from leaking
        self.assertAfterWithGeneratorInvariantsNoError(foo)


print('read')
