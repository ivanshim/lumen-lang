# tests/python/test_decorators.py:107
if False:
    def test_dotted(self):
        decorators = MiscDecorators()
        @decorators.author('Cleese')
        def foo(): return 42
        self.assertEqual(foo(), 42)
        self.assertEqual(foo.author, 'Cleese')


print('read')
