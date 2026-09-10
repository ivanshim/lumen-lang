# tests/python/test_with.py:370
if False:
    def testSingleArgBoundToMultipleElementTupleError(self):
        def shouldThrowValueError():
            with Nested(mock_contextmanager_generator()) as (foo, bar):
                pass
        self.assertRaises(ValueError, shouldThrowValueError)


print('read')
