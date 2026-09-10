# tests/python/test_with.py:652
if False:
    def testWithRaise(self):
        counter = 0
        try:
            counter += 1
            with mock_contextmanager_generator():
                counter += 10
                raise RuntimeError
            counter += 100 # Not reached
        except RuntimeError:
            self.assertEqual(counter, 11)
        else:
            self.fail("Didn't raise RuntimeError")



print('read')
