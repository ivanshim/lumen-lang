# tests/python/test_decorators.py:294
if False:
    def test_bound_function_inside_classmethod(self):
        class A:
            def foo(self, cls):
                return 'spam'

        class B:
            bar = classmethod(A().foo)

        self.assertEqual(B.bar(), 'spam')



print('read')
