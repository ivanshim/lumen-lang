# tests/python/test_decorators.py:307
if False:
    def test_simple(self):
        def plain(x):
            x.extra = 'Hello'
            return x
        @plain
        class C(object): pass
        self.assertEqual(C.extra, 'Hello')


print('read')
