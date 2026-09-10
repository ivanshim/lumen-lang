# tests/python/test_with.py:301
if False:
    def testUnboundGenerator(self):
        mock = mock_contextmanager_generator()
        with mock:
            pass
        self.assertAfterWithManagerInvariantsNoError(mock)


print('read')
