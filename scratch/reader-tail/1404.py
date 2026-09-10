# tests/python/test_with.py:611
if False:
    def testWithBreak(self):
        counter = 0
        while True:
            counter += 1
            with mock_contextmanager_generator():
                counter += 10
                break
            counter += 100 # Not reached
        self.assertEqual(counter, 11)


print('read')
