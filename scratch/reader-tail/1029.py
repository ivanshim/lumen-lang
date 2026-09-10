# tests/python/test_grammar.py:420
if False:
    def test_var_annot_simple_exec(self):
        gns = {}; lns = {}
        exec("'docstring'\n"
             "x: int = 5\n", gns, lns)
        self.assertNotIn('__annotate__', gns)

        gns.update(lns)  # __annotate__ looks at globals
        self.assertEqual(lns["__annotate__"](annotationlib.Format.VALUE), {'x': int})


print('read')
