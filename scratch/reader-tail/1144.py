# tests/python/test_set.py:660
if False:
    def test_unhashable_element(self):
        myset = {'a'}
        elem = [1, 2, 3]

        def check_unhashable_element():
            msg = "cannot use 'list' as a set element (unhashable type: 'list')"
            return self.assertRaisesRegex(TypeError, re.escape(msg))

        with check_unhashable_element():
            elem in myset
        with check_unhashable_element():
            myset.add(elem)
        with check_unhashable_element():
            myset.discard(elem)

        # Only TypeError exception is overridden,
        # other exceptions are left unchanged.
        class HashError:
            def __hash__(self):
                raise KeyError('error')

        elem2 = HashError()
        with self.assertRaises(KeyError):
            elem2 in myset
        with self.assertRaises(KeyError):
            myset.add(elem2)
        with self.assertRaises(KeyError):
            myset.discard(elem2)


print('read')
