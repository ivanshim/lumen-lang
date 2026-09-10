# tests/python/test_with.py:794
if False:
    def testExceptionInExit(self):
        body_executed = False
        with self.Dummy(gobble=True) as a, self.ExitRaises():
            body_executed = True
        self.assertTrue(a.enter_called)
        self.assertTrue(a.exit_called)
        self.assertTrue(body_executed)
        self.assertNotEqual(a.exc_info[0], None)


print('read')
