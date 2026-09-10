# tests/python/test_with.py:27
if False:
    def __init__(self, *args):
        super().__init__(*args)
        self.enter_called = False
        self.exit_called = False
        self.exit_args = None


print('read')
