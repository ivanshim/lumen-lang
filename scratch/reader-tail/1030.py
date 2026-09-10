# tests/python/test_grammar.py:429
if False:
    def test_var_annot_rhs(self):
        ns = {}
        exec('x: tuple = 1, 2', ns)
        self.assertEqual(ns['x'], (1, 2))
        stmt = ('def f():\n'
                '    x: int = yield')
        exec(stmt, ns)
        self.assertEqual(list(ns['f']()), [None])

        ns = {"a": 1, 'b': (2, 3, 4), "c":5, "Tuple": typing.Tuple}
        exec('x: Tuple[int, ...] = a,*b,c', ns)
        self.assertEqual(ns['x'], (1, 2, 3, 4, 5))


print('read')
