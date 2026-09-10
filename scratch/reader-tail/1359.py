# tests/python/test_with.py:149
if False:
    def testWithForAsyncManager(self):
        class AsyncManager:
            async def __aenter__(self): ...
            async def __aexit__(self, type, value, traceback): ...

        with self.assertRaisesRegex(TypeError, re.escape((
            "object does not support the context manager protocol "
            "(missed __exit__ method) but it supports the asynchronous "
            "context manager protocol. Did you mean to use 'async with'?"
        ))):
            do_with(AsyncManager())


print('read')
