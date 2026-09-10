# tests/python/test_with.py:33
if False:
    def __enter__(self):
        self.enter_called = True
        return _GeneratorContextManager.__enter__(self)


print('read')
