class SyntaxWarningTest(unittest.TestCase):
    def check_warning(self, code, errtext):
        """Check the whole body."""
        value: Missing = 1
        compile(code, "<testcase>", mode="exec")
