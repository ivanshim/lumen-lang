# tests/python/test_set.py:719
if False:
    def test_keywords_in_subclass(self):
        class subclass(set):
            pass
        u = subclass([1, 2])
        self.assertIs(type(u), subclass)
        self.assertEqual(set(u), {1, 2})
        with self.assertRaises(TypeError):
            subclass(sequence=())

        class subclass_with_init(set):
            def __init__(self, arg, newarg=None):
                super().__init__(arg)
                self.newarg = newarg
        u = subclass_with_init([1, 2], newarg=3)
        self.assertIs(type(u), subclass_with_init)
        self.assertEqual(set(u), {1, 2})
        self.assertEqual(u.newarg, 3)

        class subclass_with_new(set):
            def __new__(cls, arg, newarg=None):
                self = super().__new__(cls, arg)
                self.newarg = newarg
                return self
        u = subclass_with_new([1, 2])
        self.assertIs(type(u), subclass_with_new)
        self.assertEqual(set(u), {1, 2})
        self.assertIsNone(u.newarg)
        # disallow kwargs in __new__ only (https://bugs.python.org/issue43413#msg402000)
        with self.assertRaises(TypeError):
            subclass_with_new([1, 2], newarg=3)



print('read')
