# tests/python/test_grammar.py:401
if False:
    def test_var_annot_module_semantics(self):
        self.assertEqual(test.__annotations__, {})
        self.assertEqual(ann_module.__annotations__,
                         {'x': int, 'y': str, 'f': typing.Tuple[int, int], 'u': int | float})
        self.assertEqual(ann_module.M.__annotations__,
                         {'o': type})
        self.assertEqual(ann_module2.__annotations__, {})


print('read')
