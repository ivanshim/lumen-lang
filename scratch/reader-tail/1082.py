# tests/python/test_grammar.py:2054
if False:
    def test_complex_lambda(self):
        def test1(foo, bar):
            return ""

        def test2():
            return f"{test1(
                foo=lambda: '、、、、、、、、、、、、、、、、、',
                bar=lambda: 'abcdefghijklmnopqrstuvwxyz 123456789 123456789',
            )}"

        self.assertEqual(test2(), "")



print('read')
