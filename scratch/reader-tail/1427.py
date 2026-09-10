# tests/python/test_decorators.py:73
if False:
    def test_single(self):
        class C(object):
            @staticmethod
            def foo(): return 42
        self.assertEqual(C.foo(), 42)
        self.assertEqual(C().foo(), 42)


print('read')
