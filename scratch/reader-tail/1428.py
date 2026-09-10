# tests/python/test_decorators.py:80
if False:
    def check_wrapper_attrs(self, method_wrapper, format_str):
        def func(x):
            return x
        wrapper = method_wrapper(func)

        self.assertIs(wrapper.__func__, func)
        self.assertIs(wrapper.__wrapped__, func)

        for attr in ('__module__', '__qualname__', '__name__',
                     '__doc__', '__annotations__'):
            self.assertIs(getattr(wrapper, attr),
                          getattr(func, attr))

        self.assertEqual(repr(wrapper), format_str.format(func))
        return wrapper


print('read')
