# tests/python/test_with.py:783
if False:
    def testExceptionInEnter(self):
        try:
            with self.Dummy() as a, self.EnterRaises():
                self.fail('body of bad with executed')
        except RuntimeError:
            pass
        else:
            self.fail('RuntimeError not reraised')
        self.assertTrue(a.enter_called)
        self.assertTrue(a.exit_called)


print('read')
