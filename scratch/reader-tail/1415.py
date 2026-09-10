# tests/python/test_with.py:774
if False:
    def testExceptionInExprList(self):
        try:
            with self.Dummy() as a, self.InitRaises():
                pass
        except RuntimeError:
            pass
        self.assertTrue(a.enter_called)
        self.assertTrue(a.exit_called)


print('read')
