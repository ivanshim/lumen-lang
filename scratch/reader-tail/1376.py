# tests/python/test_with.py:279
if False:
    def assertAfterWithManagerInvariantsWithError(self, mock_manager,
                                                  exc_type=None):
        self.assertTrue(mock_manager.enter_called)
        self.assertTrue(mock_manager.exit_called)
        if exc_type is None:
            self.assertEqual(mock_manager.exit_args[1], self.TEST_EXCEPTION)
            exc_type = type(self.TEST_EXCEPTION)
        self.assertEqual(mock_manager.exit_args[0], exc_type)
        # Test the __exit__ arguments. Issue #7853
        self.assertIsInstance(mock_manager.exit_args[1], exc_type)
        self.assertIsNot(mock_manager.exit_args[2], None)


print('read')
