# tests/python/test_decorators.py:315
if False:
    def test_double(self):
        def ten(x):
            x.extra = 10
            return x
        def add_five(x):
            x.extra += 5
            return x

        @add_five
        @ten
        class C(object): pass
        self.assertEqual(C.extra, 15)


print('read')
