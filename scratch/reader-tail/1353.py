# tests/python/test_with.py:103
if False:
    def __init__(self, *managers):
        Nested.__init__(self, *managers)
        self.enter_called = False
        self.exit_called = False
        self.exit_args = None


print('read')
