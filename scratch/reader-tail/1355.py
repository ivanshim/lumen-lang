# tests/python/test_with.py:113
if False:
    def __exit__(self, *exc_info):
        self.exit_called = True
        self.exit_args = exc_info
        return Nested.__exit__(self, *exc_info)



print('read')
