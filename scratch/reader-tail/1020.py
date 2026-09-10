# tests/python/test_grammar.py:249
if False:
    def test_eof_error(self):
        samples = ("def foo(", "\ndef foo(", "def foo(\n")
        for s in samples:
            with self.assertRaises(SyntaxError) as cm:
                compile(s, "<test>", "exec")
            self.assertIn("was never closed", str(cm.exception))


print('read')
