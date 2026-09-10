# tests/python/test_with.py:415
if False:
    def testSingleResource(self):
        cm = mock_contextmanager_generator()
        def shouldThrow():
            with cm as self.resource:
                self.assertInWithManagerInvariants(cm)
                self.assertInWithGeneratorInvariants(self.resource)
                self.raiseTestException()
        self.assertRaises(RuntimeError, shouldThrow)
        self.assertAfterWithManagerInvariantsWithError(cm)
        self.assertAfterWithGeneratorInvariantsWithError(self.resource)


print('read')
