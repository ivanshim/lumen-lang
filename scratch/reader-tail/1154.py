# tests/python/test_set.py:831
if False:
    def test_keywords_in_subclass(self):
        class subclass(frozenset):
            pass
        u = subclass([1, 2])
        self.assertIs(type(u), subclass)
        self.assertEqual(set(u), {1, 2})
        with self.assertRaises(TypeError):
            subclass(sequence=())

        class subclass_with_init(frozenset):
            def __init__(self, arg, newarg=None):
                self.newarg = newarg
        u = subclass_with_init([1, 2], newarg=3)
        self.assertIs(type(u), subclass_with_init)
        self.assertEqual(set(u), {1, 2})
        self.assertEqual(u.newarg, 3)

        class subclass_with_new(frozenset):
            def __new__(cls, arg, newarg=None):
                self = super().__new__(cls, arg)
                self.newarg = newarg
                return self
        u = subclass_with_new([1, 2], newarg=3)
        self.assertIs(type(u), subclass_with_new)
        self.assertEqual(set(u), {1, 2})
        self.assertEqual(u.newarg, 3)


print('read')
