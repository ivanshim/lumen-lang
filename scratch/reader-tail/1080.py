# tests/python/test_grammar.py:2004
if False:
    def test_async_for(self):
        class Done(Exception): pass

        class AIter:
            def __aiter__(self):
                return self
            async def __anext__(self):
                raise StopAsyncIteration

        async def foo():
            async for i in AIter():
                pass
            async for i, j in AIter():
                pass
            async for i in AIter():
                pass
            else:
                pass
            raise Done

        with self.assertRaises(Done):
            foo().send(None)


print('read')
