# tests/python/test_grammar.py:121
if False:
    def test_bad_numerical_literals(self):
        check = self.check_syntax_error
        check("0b12", "invalid digit '2' in binary literal")
        check("0b1_2", "invalid digit '2' in binary literal")
        check("0b2", "invalid digit '2' in binary literal")
        check("0b1_", "invalid binary literal")
        check("0b", "invalid binary literal")
        check("0o18", "invalid digit '8' in octal literal")
        check("0o1_8", "invalid digit '8' in octal literal")
        check("0o8", "invalid digit '8' in octal literal")
        check("0o1_", "invalid octal literal")
        check("0o", "invalid octal literal")
        check("0x1_", "invalid hexadecimal literal")
        check("0x", "invalid hexadecimal literal")
        check("1_", "invalid decimal literal")
        check("012",
              "leading zeros in decimal integer literals are not permitted; "
              "use an 0o prefix for octal integers")
        check("1.2_", "invalid decimal literal")
        check("1e2_", "invalid decimal literal")
        check("1e+", "invalid decimal literal")


print('read')
