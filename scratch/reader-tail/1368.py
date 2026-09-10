# tests/python/test_with.py:237
if False:
    def testExitThrows(self):
        class ExitThrows(object):
            def __enter__(self):
                return
            def __exit__(self, *args):
                raise RuntimeError(42)
        def shouldThrow():
            with ExitThrows():
                pass
        self.assertRaises(RuntimeError, shouldThrow)



print('read')
