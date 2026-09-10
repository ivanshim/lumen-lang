# tests/python/test_with.py:621
if False:
    def testWithContinue(self):
        counter = 0
        while True:
            counter += 1
            if counter > 2:
                break
            with mock_contextmanager_generator():
                counter += 10
                continue
            counter += 100 # Not reached
        self.assertEqual(counter, 12)


print('read')
