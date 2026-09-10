# tests/python/test_decorators.py:96
if False:
    def test_staticmethod(self):
        wrapper = self.check_wrapper_attrs(staticmethod, '<staticmethod({!r})>')

        # bpo-43682: Static methods are callable since Python 3.10
        self.assertEqual(wrapper(1), 1)


print('read')
