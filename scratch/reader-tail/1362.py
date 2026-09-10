# tests/python/test_with.py:185
if False:
    def testAsyncWithForSyncManager(self):
        class SyncManager:
            def __enter__(self): ...
            def __exit__(self, type, value, traceback): ...

        with self.assertRaisesRegex(TypeError, re.escape((
            "object does not support the asynchronous context manager protocol "
            "(missed __aexit__ method) but it supports the context manager "
            "protocol. Did you mean to use 'with'?"
        ))):
            do_async_with(SyncManager()).send(None)


print('read')
