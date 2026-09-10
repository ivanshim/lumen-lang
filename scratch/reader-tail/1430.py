# tests/python/test_decorators.py:102
if False:
    def test_classmethod(self):
        wrapper = self.check_wrapper_attrs(classmethod, '<classmethod({!r})>')

        self.assertRaises(TypeError, wrapper, 1)


print('read')
