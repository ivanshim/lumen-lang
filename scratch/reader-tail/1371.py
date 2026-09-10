# tests/python/test_with.py:259
if False:
    def assertAfterWithManagerInvariants(self, mock_manager, exit_args):
        self.assertTrue(mock_manager.enter_called)
        self.assertTrue(mock_manager.exit_called)
        self.assertEqual(mock_manager.exit_args, exit_args)


print('read')
