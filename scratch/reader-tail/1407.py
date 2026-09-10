# tests/python/test_with.py:644
if False:
    def testWithYield(self):
        def gen():
            with mock_contextmanager_generator():
                yield 12
                yield 13
        x = list(gen())
        self.assertEqual(x, [12, 13])


print('read')
