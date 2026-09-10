# tests/python/test_with.py:722
if False:
    def testExitFalseDoesntSwallowException(self):
        class EuropeanSwallow:
            def __enter__(self): pass
            def __exit__(self, t, v, tb): return False
        try:
            with EuropeanSwallow():
                1/0
        except ZeroDivisionError:
            pass
        else:
            self.fail("ZeroDivisionError should have been raised")



print('read')
