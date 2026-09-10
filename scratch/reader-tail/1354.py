# tests/python/test_with.py:109
if False:
    def __enter__(self):
        self.enter_called = True
        return Nested.__enter__(self)


print('read')
