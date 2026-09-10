# tests/python/test_decorators.py:328
if False:
    def test_order(self):
        def applied_first(x):
            x.extra = 'first'
            return x
        def applied_second(x):
            x.extra = 'second'
            return x
        @applied_second
        @applied_first
        class C(object): pass
        self.assertEqual(C.extra, 'second')


print('read')
