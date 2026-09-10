# tests/python/test_grammar.py:755
if False:
    def test_former_statements_refer_to_builtins(self):
        keywords = "print", "exec"
        # Cases where we want the custom error
        cases = [
            "{} foo",
            "{} {{1:foo}}",
            "if 1: {} foo",
            "if 1: {} {{1:foo}}",
            "if 1:\n    {} foo",
            "if 1:\n    {} {{1:foo}}",
        ]
        for keyword in keywords:
            custom_msg = "call to '{}'".format(keyword)
            for case in cases:
                source = case.format(keyword)
                with self.subTest(source=source):
                    with self.assertRaisesRegex(SyntaxError, custom_msg):
                        exec(source)
                source = source.replace("foo", "(foo.)")
                with self.subTest(source=source):
                    with self.assertRaisesRegex(SyntaxError, "invalid syntax"):
                        exec(source)


print('read')
