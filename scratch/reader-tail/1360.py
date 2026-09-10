# tests/python/test_with.py:161
if False:
    def testAsyncEnterAttributeError(self):
        class LacksAsyncEnter:
            async def __aexit__(self, type, value, traceback): ...

        with self.assertRaisesRegex(TypeError, re.escape((
            "object does not support the asynchronous context manager protocol "
            "(missed __aenter__ method)"
        ))):
            do_async_with(LacksAsyncEnter()).send(None)


print('read')
