# tests/python/test_with.py:37
if False:
    def __exit__(self, type, value, traceback):
        self.exit_called = True
        self.exit_args = (type, value, traceback)
        return _GeneratorContextManager.__exit__(self, type,
                                                 value, traceback)



print('read')
