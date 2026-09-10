# tests/python/test_with.py:171
if False:
    def testAsyncExitAttributeError(self):
        class LacksAsyncExit:
            async def __aenter__(self): ...

        msg = re.escape((
            "object does not support the asynchronous context manager protocol "
            "(missed __aexit__ method)"
        ))
        # a missing __aexit__ is reported missing before a missing __aenter__
        with self.assertRaisesRegex(TypeError, msg):
            do_async_with(object()).send(None)
        with self.assertRaisesRegex(TypeError, msg):
            do_async_with(LacksAsyncExit()).send(None)


print('read')
