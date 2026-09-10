# tests/python/test_grammar.py:234
if False:
    def test_bytes_prefixes(self):
        def check(s):
            parsed = eval(s)
            self.assertIs(type(parsed), bytes)
            self.assertGreater(len(parsed), 0)

        check("b'abc'")
        check("br'abc\t'")
        check("rb'abc\a'")


print('read')
