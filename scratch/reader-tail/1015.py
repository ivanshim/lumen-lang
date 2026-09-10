# tests/python/test_grammar.py:143
if False:
    def test_end_of_numerical_literals(self):
        def check(test, error=False):
            with self.subTest(expr=test):
                if error:
                    with warnings.catch_warnings(record=True) as w:
                        with self.assertRaisesRegex(SyntaxError,
                                    r'invalid \w+ literal'):
                            compile(test, "<testcase>", "eval")
                    self.assertEqual(w,  [])
                else:
                    self.check_syntax_warning(test,
                            errtext=r'invalid \w+ literal')

        for num in "0xf", "0o7", "0b1", "9", "0", "1.", "1e3", "1j":
            compile(num, "<testcase>", "eval")
            check(f"{num}and x", error=(num == "0xf"))
            check(f"{num}or x", error=(num == "0"))
            check(f"{num}in x")
            check(f"{num}not in x")
            check(f"{num}if x else y")
            check(f"x if {num}else y", error=(num == "0xf"))
            check(f"[{num}for x in ()]")
            check(f"{num}spam", error=True)

            # gh-88943: Invalid non-ASCII character following a numerical literal.
            with self.assertRaisesRegex(SyntaxError, r"invalid character '⁄' \(U\+2044\)"):
                compile(f"{num}⁄7", "<testcase>", "eval")

            with self.assertWarnsRegex(SyntaxWarning, r'invalid \w+ literal'):
                compile(f"{num}is x", "<testcase>", "eval")
            with warnings.catch_warnings():
                warnings.simplefilter('error', SyntaxWarning)
                with self.assertRaisesRegex(SyntaxError,
                            r'invalid \w+ literal'):
                    compile(f"{num}is x", "<testcase>", "eval")

        check("[0x1ffor x in ()]")
        check("[0x1for x in ()]")
        check("[0xfor x in ()]")


print('read')
