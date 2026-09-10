# tests/python/test_with.py:472
if False:
    def testNestedExceptionBeforeInnerStatement(self):
        mock_a = mock_contextmanager_generator()
        mock_b = mock_contextmanager_generator()
        self.bar = None
        def shouldThrow():
            with mock_a as self.foo:
                self.assertInWithManagerInvariants(mock_a)
                self.assertInWithGeneratorInvariants(self.foo)
                self.raiseTestException()
                with mock_b as self.bar:
                    pass
        self.assertRaises(RuntimeError, shouldThrow)
        self.assertAfterWithManagerInvariantsWithError(mock_a)
        self.assertAfterWithGeneratorInvariantsWithError(self.foo)

        # The inner statement stuff should never have been touched
        self.assertEqual(self.bar, None)
        self.assertFalse(mock_b.enter_called)
        self.assertFalse(mock_b.exit_called)
        self.assertEqual(mock_b.exit_args, None)


print('read')
