# tests/python/test_with.py:125
if False:
    def testEnterAttributeError(self):
        class LacksEnter:
            def __exit__(self, type, value, traceback): ...

        with self.assertRaisesRegex(TypeError, re.escape((
            "object does not support the context manager protocol "
            "(missed __enter__ method)"
        ))):
            do_with(LacksEnter())


print('read')
