# tests/python/test_with.py:426
if False:
    def testExceptionNormalized(self):
        cm = mock_contextmanager_generator()
        def shouldThrow():
            with cm as self.resource:
                # Note this relies on the fact that 1 // 0 produces an exception
                # that is not normalized immediately.
                1 // 0
        self.assertRaises(ZeroDivisionError, shouldThrow)
        self.assertAfterWithManagerInvariantsWithError(cm, ZeroDivisionError)


print('read')
