# tests/python/test_with.py:712
if False:
    def testExitTrueSwallowsException(self):
        class AfricanSwallow:
            def __enter__(self): pass
            def __exit__(self, t, v, tb): return True
        try:
            with AfricanSwallow():
                1/0
        except ZeroDivisionError:
            self.fail("ZeroDivisionError should have been swallowed")


print('read')
