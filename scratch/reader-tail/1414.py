# tests/python/test_with.py:767
if False:
    def testNoExceptions(self):
        with self.Dummy() as a, self.Dummy() as b:
            self.assertTrue(a.enter_called)
            self.assertTrue(b.enter_called)
        self.assertTrue(a.exit_called)
        self.assertTrue(b.exit_called)


print('read')
