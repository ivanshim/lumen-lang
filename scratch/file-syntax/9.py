class SyntaxWarningTest(unittest.TestCase):
    def check_warning(self, code, errtext):
        """Check the whole body."""
        compile(code, "<testcase>", mode="exec")
