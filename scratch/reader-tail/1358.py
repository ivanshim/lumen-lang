# tests/python/test_with.py:135
if False:
    def testExitAttributeError(self):
        class LacksExit:
            def __enter__(self): ...

        msg = re.escape((
            "object does not support the context manager protocol "
            "(missed __exit__ method)"
        ))
        # a missing __exit__ is reported missing before a missing __enter__
        with self.assertRaisesRegex(TypeError, msg):
            do_with(object())
        with self.assertRaisesRegex(TypeError, msg):
            do_with(LacksExit())


print('read')
