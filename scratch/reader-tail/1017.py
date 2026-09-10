# tests/python/test_grammar.py:223
if False:
    def test_string_prefixes(self):
        def check(s):
            parsed = eval(s)
            self.assertIs(type(parsed), str)
            self.assertGreater(len(parsed), 0)

        check("u'abc'")
        check("r'abc\t'")
        check("rf'abc\a {1 + 1}'")
        check("fr'abc\a {1 + 1}'")


print('read')
