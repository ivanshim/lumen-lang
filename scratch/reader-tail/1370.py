# tests/python/test_with.py:254
if False:
    def assertInWithManagerInvariants(self, mock_manager):
        self.assertTrue(mock_manager.enter_called)
        self.assertFalse(mock_manager.exit_called)
        self.assertEqual(mock_manager.exit_args, None)


print('read')
