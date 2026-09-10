# tests/python/test_with.py:633
if False:
    def testWithReturn(self):
        def foo():
            counter = 0
            while True:
                counter += 1
                with mock_contextmanager_generator():
                    counter += 10
                    return counter
                counter += 100 # Not reached
        self.assertEqual(foo(), 11)


print('read')
